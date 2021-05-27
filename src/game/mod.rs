use getrandom;
use ggez;
use oorandom::Rand32;

use ggez::event::{KeyCode, KeyMods};
use ggez::{event, graphics, Context, GameResult};

use std::collections::LinkedList;
use std::time::{Duration, Instant};

use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};

use std::boxed::Box;

use super::Mode;

mod concat;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Player { One, Two, }

impl Player {
    pub fn not(&self) -> Self {
        match self {
            Player::One => Player::Two,
            Player::Two => Player::One,
        }
    }
}

/* Set up some constants that will help us out later */
const BUFFER_SIZE: usize = 8;
// We choose to make a 30x20 game board
const GRID_SIZE: (i16, i16) = (30, 20);
// We define the pixel size of each tile
const GRID_CELL_SIZE: (i16, i16) = (32, 32);

// actual window size
const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32
);

// How often we want our game to update
const UPDATES_PER_SECOND: f32 = 8.0;
// And we get the milliseconds of delay that this update rate
// corresponds to
const MILLIS_PER_UPDATE: u64 = (1.0 / UPDATES_PER_SECOND * 1000.0) as u64;

pub fn start_game(stream: TcpStream, mode: Mode) -> GameResult {
    let name = match mode {
        Mode::Server => "Snake server",
        Mode::Client => "Snake client",
    };
    // Here we use a ContextBuilder to setup metadata about our game.
    let (mut ctx, mut events_loop) = ggez::ContextBuilder::new(name, "Karl")
        // Next we set up the window. 
        .window_setup(ggez::conf::WindowSetup::default().title(name))
        // Now we get to set the zize of the window which we use
        // our SCREEN_SIZE constant from earlier to help with
        .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        // and finally we attempt to build the context and create the window. If it fails, we panic with
        // the message
        .build()?;
        // Next we create a new instance of our GameState struct, which implements EventHandler
        let mut state = GameState::new(mode, stream);
        event::run(&mut ctx, &mut events_loop, &mut state)
}

// A struct that holds an entity's position on our game board
// or grid which we defined above. We'll use signed integers because we only
// want to store whole numbers, and we need to be signed so that they work
// properly with our modulus arithmetic later.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct GridPosition {
    x: i16,
    y: i16,
}

trait ModuloSigned {
    fn modulo(&self, n: Self) -> Self;
}

impl<T> ModuloSigned for T
where T: std::ops::Add<Output = T> + std::ops::Rem<Output = T> + Clone,
{
    fn modulo(&self, n: T) -> T {
        (self.clone() % n.clone() + n.clone()) % n.clone()
    }
}

impl GridPosition {
    /// We make a standard helper function so that we can create a new
    /// `GridPosition` more easily
    pub fn new(x: i16, y: i16) -> Self {
        GridPosition { x, y }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        let x_bytes = self.x.to_be_bytes();
        let y_bytes = self.y.to_be_bytes();

        concat::concat_i16_i16(&x_bytes, &y_bytes)
    }

    pub fn from_bytes(bytes: &[u8; 4]) -> GridPosition {
        let mut x_bytes : [u8; 2] = [0; 2];
        let mut y_bytes : [u8; 2] = [0; 2];

        for i in 0..4 {
            if i >= 2 {
                y_bytes[i - 2] = bytes[i];
            } else {
                x_bytes[i] = bytes[i];
            }
        }

        let x = i16::from_be_bytes(x_bytes);
        let y = i16::from_be_bytes(y_bytes);

        Self::new(x, y)
    }

    /// As well as a helper function that will give us a random 
    /// `GridPosition` from `(0, 0)` to `(max_x, max_y)`.
    pub fn random(rng: &mut Rand32, max_x: i16, max_y: i16) -> Self {
        // We can use `into()` to convert from `(i16, i16)` to a `GridPosition`
        // since we implement `From<(i16, i16)>` for GridPosition below. 
        (
            rng.rand_range(0..(max_x as u32)) as i16,
            rng.rand_range(0..(max_y as u32)) as i16
        ).into()
    }

    /// We'll make another helper function that takes one grid position and returns
    /// a new one after making one move in the direction of `dir`. We use
    /// our `SignedModulo` trait above, which is now implemented on `i16` because 
    /// it satisfies the trait bounds, to automatically wrap around within our grid
    /// size if the move would have otherwise moved us off the board to the top,
    /// bottom, left, or right.
    pub fn new_from_move(pos: GridPosition, dir: Direction) -> Self {
        match dir {
            Direction::Up    => GridPosition::new(pos.x, (pos.y - 1).modulo(GRID_SIZE.1)),
            Direction::Down  => GridPosition::new(pos.x, (pos.y + 1).modulo(GRID_SIZE.1)),
            Direction::Left  => GridPosition::new((pos.x - 1).modulo(GRID_SIZE.0), pos.y),
            Direction::Right => GridPosition::new((pos.x + 1).modulo(GRID_SIZE.0), pos.y),
        }
    }
}

/// We implement the `From` trait, which in this case allows us to convert easily
/// between a GridPosition and a ggez `graphics::Rect` which fills that grid cell.
/// Now we can just call `into()` on a GridPosition where we want a `Rect` that
/// represents that grid cell.
impl From<GridPosition> for graphics::Rect {
    fn from(pos: GridPosition) -> graphics::Rect {
        graphics::Rect::new_i32(
            pos.x as i32 * GRID_CELL_SIZE.0 as i32,
            pos.y as i32 * GRID_CELL_SIZE.1 as i32,
            GRID_CELL_SIZE.0 as i32,
            GRID_CELL_SIZE.1 as i32,
        )
    }

    // fn to_be_bytes(&self) {
    //     let x_bytes = self.x.to_be_bytes();
    //     let y_bytes = self.y.to_be_bytes();
    // }
}

/// And here, we implement `From` again to allow us to easily convert between 
/// `(i16, i16)` and a GridPosition
impl From<(i16, i16)> for GridPosition {
    fn from(pos: (i16, i16)) -> Self {
        GridPosition{ x: pos.0, y: pos.1 }
    }
}

/// Next we create an enum that will represent all the possible
/// directions that our snake could move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right
}

impl Direction {
    /// We create a helper function that will allow us to easily get the inverse 
    /// of a `Direction` which we can use later to check if the player should 
    /// be able to move the snake in a certain direction.
    pub fn inverse(&self) -> Self {
        match *self {
            Direction::Up    => Direction::Down,
            Direction::Down  => Direction::Up,
            Direction::Left  => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    pub fn to_bytes(&self) -> [u8; 1] {
        match *self {
            Direction::Up    => [0],
            Direction::Down  => [1],
            Direction::Left  => [2],
            Direction::Right => [3],
        }
    }

    pub fn from_bytes(bytes: &[u8; 1]) -> Direction {
        match *bytes {
            [0] => Direction::Up,
            [1] => Direction::Down,
            [2] => Direction::Left,
            [3] => Direction::Right,
             _  => panic!("Error"),
        }
    }

    /// We also create a helper function that will let us convert between a
    /// `ggez` KeyCode and the Direction that it represents. Of course,
    /// not every keycde represents a direction, so we return `None` if this
    /// is the case.
    pub fn from_keycode(key: KeyCode) -> Option<Direction> {
        match key {
            KeyCode::Up    => Some(Direction::Up),
            KeyCode::Down  => Some(Direction::Down),
            KeyCode::Left  => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            _              => None,
        }
    }
}

/// This is mostly just a semantic abstraction over a `GridPosition` to represent
/// a segment of the snake. It could be useful to, say, have each segment contain
/// its own color or something similar. 
#[derive(Clone, Copy, Debug)]
struct Segment {
    pos: GridPosition,
}

impl Segment {
    pub fn new(pos: GridPosition) -> Self {
        Segment { pos }
    }
}

/// This is again an abstraction over a GridPosition that represents a
/// piece of food the snake can eat. It can draw itself.
struct Food {
    pos: GridPosition,
}

impl Food {
    pub fn new(pos: GridPosition) -> Self {
        Food { pos }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.pos.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8; 4]) -> Self {
        let grid_pos = GridPosition::from_bytes(bytes);
        Self::new(grid_pos)
    }

    /// We have a function that takes in `&mut ggez::Context` which we use with the
    /// helpers in `ggez::graphics` to do the drawing. We also return a 
    /// `ggez::GameResult` so that we can use the `?` operator to bubble up failure
    /// of drawing.__rust_force_expr!
    /// 
    /// Note: this method of drawing does not scale. If you need to render a large
    /// number of shapes, use a SpriteBatch. This approach is fine for this example since
    /// there are a fairly limited number of calls.
    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        // First, we set the color to draw with, in this case, all the food will be
        // colored blue.
        let color = [0.0, 0.0, 1.0, 1.0].into();
        // then we draw a rectangle with the Fill draw mode, and we convert the food's
        // position into a `ggez::Rect` using `.into()` which we can do since we implemented
        // `From<GridPosition>` for `Rect` earlier.
        let rectangle = 
            graphics::Mesh::new_rectangle(
                ctx, 
                graphics::DrawMode::fill(), 
                self.pos.into(), 
                color
        )?;
        graphics::draw(
            ctx,
            &rectangle,
            (ggez::mint::Point2 {x: 0.0, y: 0.0},)
        )
    }
}

/// Here, we define an enum of the possible things that the snake could have eaten
/// during an update of the game. It could have either eaten a piece of Food, or
/// it could have eaten itself if the head ran into its body. 
#[derive(Clone, Copy, Debug)]
enum Ate {
    Itself,
    Food,
}

/// Now we make a struct that contains all the information needed to describe the 
/// state of the Snake itself.
struct Snake {
    /// First we have the head of the snake, which is a single `Segment`.
    head: Segment,
    /// Then we have the current direction the snake is moving. This is 
    /// the direction it will move when `update` is called on it.
    dir: Direction,
    /// Next we have the body, which we choose to represent as a `LinkedList`
    /// of `Segment`s.
    body: LinkedList<Segment>,
    /// Now we have a property that represents the result of the update
    /// that was performed. The snake could have eaten nothing (None), 
    /// Food (Some(Ate::FOod)),
    /// or Itself (Some(Ate::Itself))
    ate: Option<Ate>,
    /// Finally we store the direction that the snake was traveling the last
    /// time that update was called, which we will use to determine valid
    /// directions that it could move the next time update is called.
    last_update_dir: Direction,
    /// Store the direction that will be used in the `Update` after the next
    /// `update`. This is needed so a user can press two directions (left then up)
    /// before one `update` has happened. It sort of queues up key press input
    next_dir: Option<Direction>,
    /// The color of the snake's head
    head_color: graphics::Color,
    /// The color of the snake's body
    body_color: graphics::Color,
}

impl Snake {
    pub fn new(pos: GridPosition, player: Player) -> Self {
        let mut body = LinkedList::new();
        let body_color: graphics::Color;
        let head_color: graphics::Color;
        // Set the colors
        match player {
            Player::One => {
                head_color = [0.3, 0.3, 0.0, 1.0].into();
                body_color = [1.0, 0.5, 0.0, 1.0].into();
            },
            Player::Two => {
                head_color = [0.2, 0.3, 0.4, 1.0].into();
                body_color = [0.3, 0.7, 0.2, 1.0].into();
            }
        }


        // our snake will initially have a head and one body segment,
        // and will be moving to the right.
        body.push_back(Segment::new((pos.x - 1, pos.y).into()));
        Snake {
            head: Segment::new(pos),
            dir: Direction::Right, 
            last_update_dir: Direction::Right,
            body: body,
            ate: None,
            next_dir: None,
            head_color,
            body_color,
        }
    }

    /// A helper function that determines whether the snake eats a given
    /// piece of Food based on its current position.
    fn eats(&self, food: &Food) -> bool {
        if self.head.pos == food.pos {
            true
        } else {
            false
        }
    }

    /// A helper function that determines whether the snake its itself
    /// based on its current position
    fn eats_self(&self) -> bool {
        for seg in self.body.iter() {
            if self.head.pos == seg.pos {
                return true
            }
        }
        false
    }

    /// The main update function for our snake which gets called every time
    /// we want to update the game state
    fn update(&mut self, food: &Food) {
        // If `last_update_dir` has already been update to be the same as `dir`
        // and we have a `next_dir`, then set `dir` to `next_dir` and unset
        // `next_dir`
        if self.last_update_dir == self.dir && self.next_dir.is_some() {
            self.dir = self.next_dir.unwrap();
            self.next_dir = None;
        }

        // First we get a new head position by using our `new_from_move` helper
        // function from earlier. We move our head in the direction we are
        // currently heading.
        let new_head_pos = GridPosition::new_from_move(self.head.pos, self.dir);
        // next we create a new segment will be our new head segment using the
        // new position we just made. 
        let new_head = Segment::new(new_head_pos);
        // then we push our current head segment onto the front of our body
        self.body.push_front(self.head);
        // And finally make our actual head the new Segment we created.
        // This has effectively moved the snake in the current direction.
        self.head = new_head;
        // Next we check whether the snake eats itself or some food, if so,
        // we set our `ate` member to reflect that state. 
        if self.eats_self() {
            self.ate = Some(Ate::Itself);
        } else if self.eats(food) {
            self.ate = Some(Ate::Food);
        } else {
            self.ate = None;
        }

        // If we didn't eat anything this turn, we remove the last segment
        // from our body which gives the illusion that the snake is moving.
        if let None = self.ate {
            self.body.pop_back();
        }

        // and set our last_update_dir to the direction we just moved.
        self.last_update_dir = self.dir;
    }

    /// Here we have the Snake draw itself. This is very similar to how we saw
    /// the food draw itself earlier
    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        // We first iterate through the body segments and draw them.
        for seg in self.body.iter() {
            // Again, we set the color (in this case an orangey color)
            // and then draw the rect that we convert that segment's position into.
            let rectangle = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                seg.pos.into(),
                self.body_color,
                //[0.3, 0.3, 0.0, 1.0].into(),
            )?;
            graphics::draw(ctx, &rectangle, (ggez::mint::Point2 {x:0.0, y:0.0},))?;
        }
        // And then do the same for the head, instead making it fully red to
        // distinguish it.
        let rectangle = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.head.pos.into(),
            self.head_color,
            //[1.0, 0.5, 0.0, 1.0].into(),
        )?;
        graphics::draw(ctx, &rectangle, (ggez::mint::Point2 {x: 0.0, y: 0.0 },))?;

        Ok(())
    }
}

/// Now we have the heart of our game, the GameState. This struct will implement
/// ggez's `EventHandler` trait and will therefore drive everything else that happens
/// in our game
struct GameState {
    /// We first need a snake
    player1: Snake,
    player2: Snake,
    mode: Mode,
    /// A piece of food
    food: Food,
    /// Whether the game is over or not
    gameover: bool,
    /// Our RNG state
    rng: Rand32,
    /// and we track the last time we updated so that we can limit 
    /// our update rate
    last_update: Instant,
    /// TCP Stream
    stream: TcpStream,
    update_nbr: u128,
}

impl GameState {
    /// Our new function will set up the initial state of our game.
    pub fn new(mode: Mode, mut stream: TcpStream) -> Self {
        // First we put our snake a quarter of the way accross our grid in the x axis.
        // and half way down the y axis. This works well since we start out moving to the right
        let mod_pos = GRID_SIZE.1 / 4;
        let snake_pos_2 = (GRID_SIZE.0 / 4, mod_pos + GRID_SIZE.1 / 2).into();
        let snake_pos_1 = (GRID_SIZE.0 / 4, mod_pos).into();
        // and we seed ourRNG with the system RNG.
        let mut seed: [u8; 8] = [0; 8];
        getrandom::getrandom(&mut seed[..]).expect("Could not create RNG seed");

        let food_pos;
        let mut rng = Rand32::new(u64::from_ne_bytes(seed));

        match mode {
            Mode::Server => {
                let mut buffer = [0; BUFFER_SIZE];
                food_pos = GridPosition::random(&mut rng, GRID_SIZE.0, GRID_SIZE.1);
                // Send the initial food position to the client
                buffer = concat::add_position(&mut buffer, &food_pos.to_bytes());
                let _ = stream.write(&buffer).unwrap();
            }
            Mode::Client => {
                /* Receive the initial food position */
                let mut buffer = [0; BUFFER_SIZE];
                let _ = stream.read_exact(&mut buffer).unwrap();
                let pos = concat::read_position(&buffer);
                let gp = GridPosition::from_bytes(&pos);
                food_pos = gp;
            }
        }

        GameState {
            player1: Snake::new(snake_pos_1, Player::One),
            player2: Snake::new(snake_pos_2, Player::Two),
            mode,
            food: Food::new(food_pos),
            gameover: false,
            rng,
            last_update: Instant::now(),
            stream,
            update_nbr: 0,
        }
    }
}

/// Now we implement EventHandler for GameState. This provides an interface 
/// that ggez will call automatically when different events happen.
impl event::EventHandler for GameState {
    /// Update will happen on every frame before it is drawn. This is where
    /// we update our game state to react to whatever is happening in the game
    /// world.
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        // First we check to see if enough time has elapsed since our last update
        // based on the update rate so we defined at the top
        // if not, we do nothing and return early.
        let mut buffer = [0; BUFFER_SIZE];

        if !(Instant::now() - self.last_update >= Duration::from_millis(MILLIS_PER_UPDATE)) {
            return Ok(());
        }

        // Then we check to see if the game is over. If not, we'll update. If so,
        // we just do nothing.
        if !self.gameover {
            match self.mode {
                Mode::Server => {
                    // Here we do that actual updating of our game world. First, we tell the
                    // snake to update itself,
                    // passing in a reference to our piece of food.
                    self.player1.update(&self.food);
                    
                    // Next, we check if the snake ate anything as it updated.
                    if let Some(ate) = self.player1.ate {
                        match ate {
                            Ate::Food => {
                                let new_food_pos =
                                    GridPosition::random(&mut self.rng, GRID_SIZE.0, GRID_SIZE.1);
                                self.food.pos = new_food_pos;     
                            }
                            Ate::Itself => {
                                self.gameover = true;
                            }
                        }
                    }

                    if let Some(ate) = self.player2.ate {
                        match ate {
                            Ate::Food => {
                                let new_food_pos =
                                    GridPosition::random(&mut self.rng, GRID_SIZE.0, GRID_SIZE.1);
                                self.food.pos = new_food_pos;     
                            }
                            Ate::Itself => {
                                self.gameover = true;
                            }
                        }
                    }

                    // Then send the new food location to the client
                    buffer = concat::add_position(&mut buffer, &self.food.pos.to_bytes());
                    // we also send if the game is over
                    buffer = concat::is_game_over(&mut buffer, self.gameover);
                    // We also want to send the keystroke of player 1 to the client
                    buffer = concat::write_directions(
                        &mut buffer, 
                        self.player1.dir, 
                        self.player1.last_update_dir,
                        self.player1.next_dir,
                    );
                    // Send it over to the client
                    self.stream.write(&buffer[0..BUFFER_SIZE]).unwrap();

                    // Read the buffer from the client
                    self.stream.read_exact(&mut buffer).unwrap();
                    // And now we read the actions of player2
                    let (dir, last_update_dir, next_dir) = concat::read_directions(&buffer);
                    self.player2.dir = dir;
                    self.player2.last_update_dir = last_update_dir;
                    self.player2.next_dir = next_dir;

                    self.player2.update(&self.food);
                },
                Mode::Client => {
                    // Client owns player2 so we update player 2 from client
                    
                    self.player2.update(&self.food);

                    // We get the new position of the food.
                    let _ = self.stream.read_exact(&mut buffer).unwrap();
                    let pos = concat::read_position(&buffer);
                    let gp = GridPosition::from_bytes(&pos);
                    
                    // Also, we read what player 1 did
                    let (dir, last_update_dir, next_dir) = concat::read_directions(&buffer);
                    self.player1.dir = dir;
                    self.player1.last_update_dir = last_update_dir;
                    self.player1.next_dir = next_dir;

                    self.player1.update(&self.food);
                    self.food.pos = gp;

                    // We also have to encode the keypresses of player 2
                    // and send them to the server
                    buffer = concat::write_directions(
                        &mut buffer,
                        self.player2.dir,
                        self.player2.last_update_dir,
                        self.player2.next_dir,
                    );

                    self.stream.write(&buffer[0..BUFFER_SIZE]).unwrap();
                }
            } 
        }
        // If we updated, we set our last update to be now
        self.last_update = Instant::now();
        self.update_nbr += 1;
        
        Ok(())
    }

    /// The draw is where we should actually render the game's current state.
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // First we clear the screen to a nice (well, maybe pretty glaring ;)) green
        graphics::clear(ctx, [0.0, 1.0, 0.0, 1.0].into());
        // Then we tell the snake and the food to draw themselves.
        self.player1.draw(ctx)?;
        self.player2.draw(ctx)?;
        self.food.draw(ctx)?;
        // Finally, we call graphics::present to cycle the gpu's framebuffer
        // and display the new frame we just drew.
        graphics::present(ctx)?;
        // We yield the current thread until the next update
        ggez::timer::yield_now();
        // and return success
        Ok(())
    }

    /// key_down_event gets fired when a key gets pressed
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymod: KeyMods,
        _repeat: bool) {
        
        // Here we attempt to convert the KeyCode into a direction
        if let Some(dir) = Direction::from_keycode(keycode) {
            // if it succeeds, we check if the new direction has already been set.
            // and make sure the new idrection is different then `snake.dir`
            match self.mode {
                Mode::Server => {
                    if self.player1.dir != self.player1.last_update_dir && dir.inverse() != self.player1.dir {
                        self.player1.next_dir = Some(dir);
                    } else if dir.inverse() != self.player1.last_update_dir {
                        // If no new direction has been set and the direction is not the inverse,
                        // of the last_update_dir, then set the snake's new direction to be
                        // the direction the user pressed.
                        self.player1.dir = dir;
                        // Also, we send this to the client
                        // let buffer = dir.to_bytes();
                        // let _ = self.stream.write(&buffer).unwrap();
                    }
                }
                Mode::Client => {
                    if self.player2.dir != self.player2.last_update_dir && dir.inverse() != self.player2.dir {
                        self.player2.next_dir = Some(dir);
                    } else if dir.inverse() != self.player2.last_update_dir {
                        // If no new direction has been set and the direction is not the inverse,
                        // of the last_update_dir, then set the snake's new direction to be
                        // the direction the user pressed.
                        self.player2.dir = dir;
                    }
                }
            }
        }
    }
}