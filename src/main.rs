mod connect;
mod game;

use connect::{server_main, client_main, make_ip};

use druid::{
    widget::{Button, Flex, Label, Align, TextBox},
    AppLauncher, LocalizedString,
    Widget, WidgetExt,
    WindowDesc, Data, Lens, Env
};

use std::fmt::Display;

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const HORIZONTAL_WIDGET_SPACING: f64 = 15.0;
const IP_ADD_WIDTH: f64 = 40.0;
const WINDOW_TITLE: LocalizedString<InitState> = LocalizedString::new("Snake");

fn main() {
    // We initialize the initial state first
    let state = InitState::new();

    let app_window = WindowDesc::new(build_ui)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    AppLauncher::with_window(app_window)
        .launch(state)
        .expect("Failed to launch application");
}

fn build_ui() -> impl Widget<InitState> {
    let mode_label = Label::new(|data: &InitState, _env: &Env| 
        format!("You are {}", data.mode));
    // Some buttons for choosing between the server and client
    let server_btn = Button::new("Server")
        .on_click(|_ctx, data: &mut InitState, _env| {
            // set the mode to server
            (*data).mode = Mode::Server;
        });
    let client_btn = Button::new("Client")
        .on_click(|_ctx, data: &mut InitState, _env| {
            // Set the mode to client
            (*data).mode = Mode::Client;
        });
    
    /* Some widgets for asking for the IP Address */
    let ip_label = Label::new("IP Address");
    let ip1 = TextBox::new()
        .with_placeholder("")
        .fix_width(IP_ADD_WIDTH)
        .lens(InitState::ip1);
    let ip2 = TextBox::new()
        .with_placeholder("")
        .fix_width(IP_ADD_WIDTH)
        .lens(InitState::ip2);
    let ip3 = TextBox::new()
        .with_placeholder("")
        .fix_width(IP_ADD_WIDTH)
        .lens(InitState::ip3);
    let ip4 = TextBox::new()
        .with_placeholder("")
        .fix_width(IP_ADD_WIDTH)
        .lens(InitState::ip4);
    let ip_layout = Flex::row()
        .with_child(ip_label)
        .with_spacer(HORIZONTAL_WIDGET_SPACING)
        .with_child(ip1)
        .with_child(ip2)
        .with_child(ip3)
        .with_child(ip4);
    
    // and some widgets forr the port
    let port_label = Label::new("Port");
    let port_textbox = TextBox::new()
        .with_placeholder("")
        .fix_width(100.0)
        .lens(InitState::port_nbr);
    
    let port_layout = Flex::row()
        .with_child(port_label)
        .with_spacer(HORIZONTAL_WIDGET_SPACING)
        .with_child(port_textbox);

    let enter_btn = Button::new("Connect")
        .on_click(|_ctx, data: &mut InitState, _env| {
            // Form the IP Address
            let ip = make_ip((*data).ip1.clone(), (*data).ip2.clone(), (*data).ip3.clone(), (*data).ip4.clone());
            if (*data).mode == Mode::Server {
                server_main(ip.clone(), (*data).port_nbr.clone(), data);
            } else {
                client_main(ip.clone(), (*data).port_nbr.clone());
            }
            
            (*data).connection_status = ConnectionStatus::Connecting;
        });
    let status_label = Label::new(|data: &InitState, _env: &Env| 
        format!("{}", data.connection_status));

    let layout = Flex::column()
        .with_child(mode_label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(server_btn)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(client_btn)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(ip_layout)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(port_layout)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(enter_btn)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(status_label);

    Align::centered(layout)
}

#[derive(Clone, PartialEq, Data)]
enum Mode {
    Client,
    Server,
}

#[derive(Clone, PartialEq, Data)]
enum ConnectionStatus {
    NoAction,
    Connecting,
    Connected,
}

#[derive(Clone, PartialEq, Data, Lens)]
pub struct InitState {
    mode: Mode,
    connection_status: ConnectionStatus,
    ip1: String,
    ip2: String,
    ip3: String,
    ip4: String,
    port_nbr: String,
}

impl Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::NoAction => write!(f, ""),
            ConnectionStatus::Connected => write!(f, "connected"),
            ConnectionStatus::Connecting => write!(f, "waiting..."),
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Client => write!(f, "Client"),
            Mode::Server => write!(f, "Server"),
        }
    }
}

impl InitState {
    fn new() -> Self {
        InitState {
            mode: Mode::Server,
            connection_status: ConnectionStatus::NoAction,
            ip1: "0".into(),
            ip2: "0".into(),
            ip3: "0".into(),
            ip4: "0".into(),
            port_nbr: "9999".into(),
        }
    }
}