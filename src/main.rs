use std::{
    io::{stdout, stdin, Write},
    thread,
    sync::mpsc::{
        channel,
        Sender,
    }
};
use termion::{
    raw::IntoRawMode,
    screen::AlternateScreen,
    input::MouseTerminal,
    event::Key,
};
use tui::{
    Terminal,
    backend::TermionBackend,
    widgets::{Block, Borders, List, ListItem},
};
use Screen::*;
use fetch_data::{
    fetch_data::{
        fetch_new_videos,
        read_history,
        write_history,
    },
};

use data_types::{
    internal::{
        ChannelList,
        Channel,
        Video,
        ToSpans,
    },
};

mod draw;
use draw::draw;

mod events;
use events::*;

pub struct App<W: Write> {
    pub terminal: Terminal<TermionBackend<W>>,
    current_screen: Screen,
    pub app_title: String,
    pub channel_list: ChannelList,
    /* backup_list: ChannelList, */
    pub current_selected: usize,
    pub update_line: String,
    pub config: Config,
}

#[allow(dead_code)]
pub struct Config {
    show_empty_channels: bool,
}

impl<W: Write> App<W> {
    fn update(&mut self) {
        draw(self);
    }
    //--------------
    fn get_selected_channel(&mut self) -> &mut Channel {
        let i = self.current_selected;
        &mut self.channel_list.channels[i]
    }
    fn get_selected_video(&mut self) -> &mut Video {
        let c = self.get_selected_channel();
        let i = c.list_state.selected().unwrap();
        &mut c.videos[i]
    }
    //---------------
    fn open_selected_channel(&mut self) {
        self.current_selected = match self.channel_list.list_state.selected() {
            Some(selected) => selected,
            None => return,
        };
        self.current_screen = Videos;
        self.channel_list.list_state.select(None);
        self.get_selected_channel().list_state.select(Some(0));
        self.update();
    }
    fn open_selected_video(&mut self) {
        self.get_selected_video().open();
    }
    //---------------------
    fn close_right_block(&mut self) {
        self.current_screen = Channels;
        self.channel_list.list_state.select(Some(self.current_selected));
        self.update();
    }
    fn save(&self) {
        write_history(&self.channel_list);
    }
}

#[derive(PartialEq, Clone)]
enum Screen {
    Channels,
    Videos,
}


const TITLE: &str = "Terminal-Youtube";

fn update_channel_list(result_sender: Sender<ChannelList>, url_sender: Sender<String>) {
    thread::spawn(move|| {
        let new_chan = fetch_new_videos(url_sender);
        result_sender.send(new_chan.clone()).unwrap();
        write_history(&new_chan);
    });

}

fn main() {
    let stdout = stdout().into_raw_mode().unwrap();
    let mouse_terminal = MouseTerminal::from(stdout);
    /* let screen = mouse_terminal; */
    let screen = AlternateScreen::from(mouse_terminal);
    let _stdin = stdin();
    let backend = TermionBackend::new(screen);
    let terminal = Terminal::new(backend).unwrap();

    let config = Config {
        show_empty_channels: true,
    };

    let mut app = App {
        terminal,
        config,
        app_title: String::from(TITLE),
        current_screen: Channels,
        channel_list: read_history(),
        /* backup_list: ChannelList::new(), */
        current_selected: 0,
        update_line: String::new(),
    };

    let events = Events::new();

    let (result_sender, result_receiver) = channel();
    let (url_sender, url_receiver) = channel();

    update_channel_list(result_sender.clone(), url_sender.clone());
    let mut update = true;

    loop {
        let event = events.next();

        if update {
            match result_receiver.try_recv() {
                Ok(v) => {
                    app.channel_list = v;
                    /* update = false; */
                    app.update();
                },
                Err(_) => {}
            }
        }

        match event.unwrap() {
            Event::Input(input) => match input {
                Key::Char('q') => {
                    match app.current_screen {
                        Channels => {
                            break;
                        },
                        Videos => {
                            app.close_right_block();
                        },
                    }
                },
                Key::Esc | Key::Char('h') | Key::Left => {
                match app.current_screen {
                    Channels => {},
                        Videos => {
                            app.close_right_block();
                        }
                    }
                }
                Key::Char('j') | Key::Down => {
                    match app.current_screen {
                        Channels => {
                            app.channel_list.next();
                        },
                        Videos => {
                            app.get_selected_channel().next();
                        }
                    }
                    app.update();
                },
                Key::Char('k') | Key::Up => {
                    match app.current_screen {
                        Channels => {
                            app.channel_list.prev();
                        },
                        Videos => {
                            app.get_selected_channel().prev();
                        }
                    }
                    app.update();
                },
                Key::Char('\n') | Key::Char('l') | Key::Right => {  // ----------- open ---------------
                    match app.current_screen {
                        Channels => {
                            app.open_selected_channel();
                        },
                        Videos => {}
                    }
                },
                Key::Char('o') => {
                    match app.current_screen {
                        Channels => {
                            app.open_selected_channel();
                        },
                        Videos => {
                            app.open_selected_video();
                        },
                    }
                }
                Key::Char('m') => { // ----------- mark ---------------
                    match app.current_screen {
                        Channels => (),
                        Videos => {
                            app.get_selected_video().mark(true);
                            app.get_selected_channel().next();
                            app.update();
                            app.save();
                        },
                    }
                },
                Key::Char('M') => { // ----------- unmark -------------
                    match app.current_screen {
                        Channels => (),
                        Videos => {
                            app.get_selected_video().mark(false);
                            app.get_selected_channel().next();
                            app.update();
                            app.save();
                        },
                    }
                },
                Key::Char('r') => {
                    update_channel_list(result_sender.clone(), url_sender.clone());
                    app.close_right_block();
                    update = true;
                }
                /* Key::Char('t') => {
                 *     app.config.show_empty_channels = !app.config.show_empty_channels;
                 *     if app.config.show_empty_channels {
                 *         app.backup_list = app.channel_list.clone();
                 *         app.channel_list = app.channel_list.clone();
                 *     } else {
                 *         app.channel_list = app.backup_list.clone();
                 *     }
                 *     app.update();
                 * } */
                _ => {}
            }
            Event::Tick => {
                app.update_line = match url_receiver.try_recv() {
                    Ok(v) => v,
                    Err(_) => String::new(),
                };
                app.update();
            }

        }
    }
}
