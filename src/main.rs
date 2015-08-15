extern crate rustbox;

use std::default::Default;
use rustbox::{RustBox,Color,Key};
use std::fs::File;
use std::io::Read;

// TODO(cgag): perhaps the rustbox instance should
// live in here, and then write/write_inverted, render, etc
// could be methods on State. 
#[derive(Clone,Copy)]
struct State<'a> {
    selected: usize,
    domains: &'a Vec<String>,
}

enum Movement { 
    Top,
    Bottom,
    Up,
    Down,
}

enum DomainStatus { Blocked, Unblocked }
struct Domain {
    domain: String,
    blocked: DomainStatus,
}

fn main() {
    let rustbox = RustBox::init(Default::default()).unwrap();
    let domains = read_hosts();

    let init_state = State { selected: 0
                           , domains:  &domains
                           };

    w_inv(&rustbox,0,0,"Press q to quit");

    render(&rustbox, &init_state);
    rustbox.present();

    let mut state = init_state;
    loop {
        let (quit, new_state) = 
            handle_event(rustbox.poll_event(false).ok().expect("fuck"), state);
        if quit { break }
        state = new_state;
        render(&rustbox, &state);
        rustbox.present();
    }
}

fn read_hosts() -> Vec<String> {
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
    s.lines()
        .take_while(|s| !s.starts_with("### End HostBlock"))
        .skip_while(|s| !s.starts_with("### HostBlock"))
        .skip(1) // drop the ### HostBlock line
        .map(|line| line.to_string())
        .collect::<Vec<String>>()
}

fn handle_event(event: rustbox::Event, state: State) -> (bool, State) {
    let mut should_quit = false;

    // TODO(cgag): avoid all these default cases returning state somehow?
    let new_state = match event {
        rustbox::Event::KeyEvent(mkey) => {
            match mkey {
                Some(key) => match key {
                    Key::Char('q') => { should_quit = true; state },
                    Key::Char('j') => { move_sel(state, Movement::Down)   },
                    Key::Char('k') => { move_sel(state, Movement::Up)     },
                    Key::Char('J') => { move_sel(state, Movement::Bottom) },
                    Key::Char('K') => { move_sel(state, Movement::Top)    }
                    _  => { state }
                },
                _ => { state }
            }
        }
        _ => { state } 
    };

    (should_quit, new_state)
}

fn move_sel(state: State, movement: Movement) -> State {
    match movement {
        Movement::Top => { State { selected: 0, domains: state.domains } }
        Movement::Bottom => { 
            State { selected: state.domains.len()
                  , domains: state.domains }
        }
        Movement::Up => {
            if state.selected == 0 {
                State { selected: state.domains.len() - 1
                      , domains: state.domains }
            } else {
                State { selected: state.selected - 1 
                      , domains: state.domains }
            }
        }
        Movement::Down => {
            if state.selected == state.domains.len() - 1 {
                State { selected: 0
                      , domains: state.domains }
            } else {
                State { selected: state.selected + 1
                      , domains: state.domains }
            }
        }
    }
}

// TODO(cgag): make these methods on rustbox?
fn w(b: &RustBox, x: usize, y: usize, text: &str) {
    b.print(x,y,rustbox::RB_BOLD, Color::White, Color::Black, text);
}

fn w_inv(b: &RustBox, x: usize, y: usize, text: &str) {
    b.print(x,y,rustbox::RB_BOLD, Color::Black, Color::White, text);
}

fn render(b: &RustBox, state: &State) {
    for (i, line) in state.domains.iter().enumerate() {
        let mut s = String::from("[ ]");
        s.extend(line.chars());
        if i == state.selected {
            w_inv(b, 0, i, &s);
        } else {
            w(b, 0, i, &s);
        }
    }
}
