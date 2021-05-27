use super::Direction;

pub fn concat_i16_i16(arr1: &[u8], arr2: &[u8]) -> [u8; 4] {
    let mut b = [0; 4];

    for i in 0..2 {
        b[i] = arr1[i];
    }

    for i in 2..4 {
        b[i] = arr2[i - 2];
    }

    b
}

pub fn add_position(buffer: &mut [u8; 8], position: &[u8; 4]) -> [u8; 8] {
    for i in 4..8 {
        buffer[i] = position[i - 4];
    }

    buffer.clone()
}

pub fn read_position(buffer: &[u8; 8]) -> [u8; 4] {
    let mut b = [0; 4];

    for i in 4..8 {
        b[i - 4] = buffer[i];
    }
    b.clone()
}

pub fn is_game_over(buffer: &mut [u8; 8], is_game_over: bool) -> [u8; 8] {
    if is_game_over {
        buffer[0] = 1;
    } else {
        buffer[0] = 0;
    }

    buffer.clone()
}

pub fn read_game_over(buffer: &[u8; 8]) -> bool {
    buffer[0] == 1
}

pub fn write_directions(
    buffer: &mut [u8; 8],
    dir: Direction,
    last_update_dir: Direction,
    next_dir: Option<Direction>
) -> [u8; 8] {
    let dir_byte = dir.to_bytes();
    let last_update_dir_byte = last_update_dir.to_bytes();
    let next_dir_byte = match next_dir {
        Some(d) => d.to_bytes(),
        None    => [4],
    };

    buffer[1] = dir_byte[0];
    buffer[2] = last_update_dir_byte[0];
    buffer[3] = next_dir_byte[0];

    buffer.clone()
}

pub fn read_directions(buffer: &[u8; 8]) -> (Direction, Direction, Option<Direction>) {
    let dir = Direction::from_bytes(&[buffer[1]]);
    let last_update_dir = Direction::from_bytes(&[buffer[2]]);
    let next_dir = match &buffer[3] {
        0 => Some(Direction::Up),
        1 => Some(Direction::Down),
        2 => Some(Direction::Left),
        3 => Some(Direction::Right),
        _ => None,
    };

    (dir, last_update_dir, next_dir)
}