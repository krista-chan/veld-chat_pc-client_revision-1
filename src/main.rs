extern crate json;
extern crate reqwest;

use async_tungstenite::{tokio::connect_async, WebSocketStream, tokio::ConnectStream};
use std::sync::Arc;
use async_mutex::Mutex;
use std::time::Duration;
// use tokio::sync::mpsc::{channel, Sender, Receiver};
use std::collections::HashMap;
use futures::prelude::*;
use url::*;
use iced::{
    button, executor, scrollable, text_input, window, Align::Center, Application, Column, Command,
    Container, Element, HorizontalAlignment, Length, Scrollable, Settings, Text, TextInput,
};

use std::io::{self};

fn get_token() -> io::Result<String> {
    println!("Please input your https://veld.chat/ token to continue");
    let mut buffer = String::new();
    let stdin = io::stdin();

    stdin.read_line(&mut buffer)?;

    if buffer.trim().len() == 0 || buffer.is_empty() {
        get_token().unwrap();
    }

    Ok(String::from(buffer.trim()))
}

#[tokio::main]
async fn main() {
    let token: String = get_token().unwrap();

    let url: Url = Url::parse("wss://api.veld.chat/").unwrap();
    let (mut ws, _) = connect_async(url)
        .await
        .unwrap();

    let cl_token = token.clone();
    let payload = json::object!{"d": {token: cl_token}, "t": 0};

    ws.send(format!("{}", payload).into()).await.unwrap();
    println!("{}", ws.next().await.unwrap().unwrap());

    let ws_lock = Arc::new(Mutex::new(ws));

    let clone_ws_lock = Arc::clone(&ws_lock);
    let hb = heartbeat(clone_ws_lock);

    let clone_ws_lock = Arc::clone(&ws_lock);
    let read = read(clone_ws_lock);

    let cl_token = token.clone();
    let connection = Connection::new(cl_token);

    let win = open_window(connection);

    let futures = future::join3(hb, read, win).await;
    futures.2.unwrap();
}

async fn heartbeat(
    ws: Arc<Mutex<WebSocketStream<ConnectStream>>>
) {
    let payload = json::object!{t: 1000, d: null};
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await;
        println!("Waiting for lock");
        ws.lock()
            .await
            .send(format!("{}", payload).into())
            .await
            .unwrap();
        println!("Sent heartbeat!");
    }
}

async fn read(
    ws: Arc<Mutex<WebSocketStream<ConnectStream>>>,
) {
    loop {
        let msg = tokio::time::timeout(
            Duration::from_millis(200),
            ws.lock().await.next()
        ).await;

        if msg.is_ok() {
            let msg = msg.unwrap().unwrap().unwrap();

            if msg.is_pong() {
                println!("Msg was pong");
                continue;
            }
            
            println!("{}", msg);
        } else {
            continue;
        }
    }
}

async fn send_message(content: String, token: String) -> std::result::Result<reqwest::Response, reqwest::Error> {
    let mut msg_payload = HashMap::new();
    msg_payload.insert("content", content);

    let res = reqwest::Client::new()
        .post(url::Url::parse("https://api.veld.chat/channels/1/messages").unwrap())
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&msg_payload)
        .send()
        .await?;
    
    Ok(res)
}

#[derive(Clone, Debug, Default)]
struct Connection {
    token: String
}

impl Connection {
    pub fn new(token: String) -> Self {
        Connection { token: token }
    }
}

#[derive(Debug)]
struct MainView {
    btn_states: BtnStates,
    text_states: TextAreaStates,
    text_values: TextAreaValues,
    scroll_states: ScrollableStates,
    messages: Vec<String>,
    connection: Connection
}

#[derive(Clone, Copy, Debug, Default)]
struct BtnStates {
    append_btn_state: button::State,
    max_btn_state: button::State,
    min_btn_state: button::State,
    close_btn_state: button::State,
}

#[derive(Clone, Debug, Default)]
struct TextAreaStates {
    message_send_state: text_input::State,
}

#[derive(Clone, Debug, Default)]
struct TextAreaValues {
    message_send_value: String,
}

#[derive(Clone, Debug, Default)]
struct ScrollableStates {
    main_scrollable_state: scrollable::State,
}

#[derive(Debug, Clone)]
pub enum Msgs {
    CloseWindow,
    LoadedMessage(String),
    SendMessage(String),
    SendStage(String),
}

pub enum Handlers {
    SetToken(String),
    Send(String),
}

impl Application for MainView {
    type Executor = executor::Default;
    type Message = Msgs;
    type Flags = Connection;

    fn new(connection: Connection) -> (MainView, Command<Self::Message>) {
        (
            MainView {
                btn_states: BtnStates { ..Default::default() },
                text_states: TextAreaStates { ..Default::default() },
                text_values: TextAreaValues { ..Default::default() },
                scroll_states: ScrollableStates { ..Default::default() },
                messages: vec!(),
                connection: connection
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("with myself | Oh no!!!!!!!!!!!")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Msgs::CloseWindow => {
                std::process::exit(0);
            }
            Msgs::LoadedMessage(h) => {
                self.messages.push(h);
                Command::none()
            }
            Msgs::SendStage(val) => {
                if val.is_empty() || val.len() == 0 {
                    return Command::none();
                } else {
                    self.text_values.message_send_value = val.clone();
                    Command::none()
                }
            }
            Msgs::SendMessage(val) => {
                if val.is_empty() || val.len() == 0 {
                    return Command::none();
                }
                self.update(Msgs::LoadedMessage(val.trim().to_string().clone()));
                self.text_values.message_send_value.clear();

                let token = self.connection.token.clone();
                
                tokio::spawn(async move {
                    let token = token.clone();
                    send_message(val, token)
                        .await
                        .unwrap();
                });

                Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<Self::Message> {
        let main_body: Element<_> = Column::new()
            .padding(20)
            .align_items(Center)
            .width(Length::Fill)
            .height(Length::FillPortion(1))
            .push(
                Text::new("VELD'S CHAT")
                    .width(Length::Fill)
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .size(100),
            )
            .into();
        let messages: Element<_> = self
            .messages
            .iter_mut()
            .enumerate()
            .fold(
                Column::new().width(Length::Fill).padding(20).spacing(20),
                |column, (_, new_message)| {
                    column.push(
                        Text::new(format!("{}", new_message))
                            .size(35)
                            .horizontal_alignment(HorizontalAlignment::Left),
                    )
                },
            )
            .into();

        let inputs: Element<_> = TextInput::new(
            &mut self.text_states.message_send_state,
            "Send a message",
            &self.text_values.message_send_value,
            Msgs::SendStage,
        )
        .on_submit(Msgs::SendMessage(
            self.text_values.message_send_value.clone(),
        ))
        .size(30)
        .width(Length::Fill)
        .padding(35)
        .into();

        let msg_scroll: Element<_> = Scrollable::new(&mut self.scroll_states.main_scrollable_state)
            .push(messages)
            .width(Length::Fill)
            .height(Length::FillPortion(4))
            .max_height(100)
            .into();

        let content = Column::new()
            .align_items(Center)
            .spacing(20)
            .push(main_body)
            .push(msg_scroll)
            .push(inputs);

        Container::new(content).into()
    }
}

async fn open_window(connection: Connection) -> iced::Result {
    let mut settings: Settings<Connection> = Settings::with_flags(connection);
    settings.window = window::Settings {
        transparent: true,
        ..Default::default()
    };
    MainView::run(settings)
}
