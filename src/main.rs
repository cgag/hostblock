extern crate rustbox;

use std::default::Default;
use rustbox::{RustBox,Color,Key};
use std::fs::File;
use std::io::Read;
use std::cmp::{min};

// TODO(cgag): perhaps the rustbox instance should
// live in here, and then write/write_inverted, render, etc
// could be methods on State. 
#[derive(Clone,Copy)]
struct State<'a> {
    selected: usize,
    domains: &'a Vec<&'a str>,
}

enum Movement { 
    Top,
    Bottom,
    Up,
    Down,
}

fn main() {

    let rustbox = RustBox::init(Default::default()).unwrap();
    let hosts_text = read_hosts();

    let init_state = State { 
                    selected: 0, 
                    domains: &hosts_text.lines().collect(),
                };

    w_inv(&rustbox,0,0,"Press q to quit");

    render(&rustbox, &init_state);
    rustbox.present();

    let mut state = init_state;
    loop {
        let newState = match rustbox.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                match key {
                    Some(Key::Char('q')) => { break; },
                    Some(Key::Char('j')) => { 
                        // if state.selected == state.domains.len() - 1 {
                        //     State { selected: 0
                        //           , domains: state.domains }
                        // } else {
                            State { selected: state.selected + 1
                                  , domains: state.domains }
                        // }
                    },
                    Some(Key::Char('k')) => { 
                        if state.selected == 0 {
                            State { selected: state.domains.len() - 1
                                  , domains: state.domains }
                        } else {
                            State { selected: state.selected - 1 
                                  , domains: state.domains }
                        }
                    },
                    Some(Key::Char('J')) => { 
                        State { selected: state.domains.len() - 1 
                              , domains: state.domains }
                    },
                    Some(Key::Char('K')) => { moveSel(state, Movement::Top) }
                    _ => { state }
                }
            }
            _ => { panic!("failed to read event, no idea why this would happen") }
        };

        state = newState;
        render(&rustbox, &state);
        rustbox.present();
    }
}

fn read_hosts() -> String {
    let mut hosts_file: File;

    let x = File::open("/etc/hosts");
    match x {
        Ok(file) => { hosts_file = file; }
        Err(_) => { 
            panic!("Couldn't access hosts file, try running with sudo.") 
        }
    }
    
    let mut s = String::new();
    hosts_file.read_to_string(&mut s).unwrap();
    s
}


fn moveSel(state: &State, movement: Movement) -> State {
    match movement {
        Top => { State { selection: 0, domains: state.domains } }
        Bottom => {}
        Up => {}
        Down => {}
    }
}

// TODO(cgag): make these methods on rustbox
fn w(b: &RustBox, x: usize, y: usize, text: &str) {
    b.print(x,y,rustbox::RB_BOLD, Color::White, Color::Black, text);
}

fn w_inv(b: &RustBox, x: usize, y: usize, text: &str) {
    b.print(x,y,rustbox::RB_BOLD, Color::Black, Color::White, text);
}

fn render(b: &RustBox, state: &State) {
    for (i, line) in state.domains.iter().enumerate() {
        if i == state.selected {
            w_inv(b, 0, i, line);
        } else {
            w(b, 0, i, line);
        }
    }
}
// fn wInv
