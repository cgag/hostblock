extern crate rustbox;

use std::default::Default;
use rustbox::{RustBox,Color,Key};
use std::fs::File;
use std::io::Read;
use std::cmp::{min};

struct State<'a>{
    selected: usize,
    lines: Vec<&'a str>,
}

fn main() {

    let rustbox = RustBox::init(Default::default()).unwrap();
    let hosts_text = read_hosts();

    let mut state = State { 
                        selected: 0, 
                        lines: hosts_text.lines().collect(),
                    };

    w_inv(&rustbox,0,0,"Press q to quit");

    render(&rustbox, &state, &hosts_text);
    rustbox.present();
    loop {
        match rustbox.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                match key {
                    Some(Key::Char('q')) => { break; },
                    Some(Key::Char('j')) => { 
                        if state.selected == state.lines.len() - 1 {
                            state.selected = 0
                        } else {
                            state.selected += 1;
                        }
                    },
                    Some(Key::Char('k')) => { 
                        if state.selected == 0 {
                            state.selected = state.lines.len() - 1;
                        } else {
                            state.selected -= 1;
                        }
                    },
                    Some(Key::Char('J')) => { 
                        state.selected = state.lines.len() - 1 
                    },
                    Some(Key::Char('K')) => { state.selected = 0 },
                    _ => {}
                }
            }
            _ => {}
        }
        render(&rustbox, &state, &hosts_text);
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

// TODO(cgag): make these methods on rustbox
fn w(b: &RustBox, x: usize, y: usize, text: &str) {
    b.print(x,y,rustbox::RB_BOLD, Color::White, Color::Black, text);
}

fn w_inv(b: &RustBox, x: usize, y: usize, text: &str) {
    b.print(x,y,rustbox::RB_BOLD, Color::Black, Color::White, text);
}

fn render(b: &RustBox, state: &State, hosts_text: &str) {
    for (i, line) in hosts_text.lines().enumerate() {
        if i == state.selected {
            w_inv(b, 0, i, line);
        } else {
            w(b, 0, i, line);
        }
    }
}
// fn wInv
