// #![feature(plugin)]
// #![plugin(clippy)]

#![feature(slice_concat_ext)]

extern crate rustbox;
extern crate unicode_segmentation;

use std::default::Default;
use std::fs;
use std::fs::File;
use std::cmp::min;
use std::io::{Read, Write};
use std::path::Path;
use std::slice::SliceConcatExt;
use rustbox::{RustBox,Color,Key};
use unicode_segmentation::UnicodeSegmentation;

// TODO(cgag): remove as many unwraps as possible
//
// TODO(cgag): Use rustbox writes to display error messages instead of just
// stderr.  Should probably do both.

// TODO(cgag): perhaps the rustbox instance should
// live in here, and then write/write_inverted, render, etc
// could be methods on State. 
// TODO(cgag): editing status: Saved/Modified
#[derive(Clone)]
struct State {
    selected: usize,
    domains:  Vec<Domain>,
    adding:   String,
    mode:     Mode,
}

#[derive(Clone)]
struct Domain {
    url: String,
    status: DomainStatus,
}

#[derive(Clone, Copy, PartialEq)]
enum DomainStatus { Blocked, Unblocked }

enum Movement { 
    Top,
    Bottom,
    Up,
    Down,
}

#[derive(Clone, Copy)] enum Mode {
    Insert,
    Normal
}

// TODO(cgag): deglobalize these?
static TOP_RIGHT: &'static str = "┐";
static VERTICAL_LINE: &'static str = "│";
static HORIZONTAL_LINE: &'static str = "─";
static TOP_LEFT: &'static str = "┌";
static BOTTOM_RIGHT: &'static str = "┘";
static BOTTOM_LEFT: &'static str = "└";
static BOX_WIDTH: usize = 40;

fn main() {
    let rustbox = RustBox::init(Default::default()).unwrap();
    let domains = parse_hosts(read_hosts());

    let init_state = State { selected: 0
                           , domains:  domains
                           , adding:   String::from("")
                           , mode:     Mode::Normal
                           };

    rustbox.draw(&init_state);

    let mut state = init_state;
    loop {
        let (quit, new_state) = 
            handle_event(rustbox.poll_event(false).ok().expect("poll failed"), 
                         &state);
        if quit { break }
        state = new_state;
        rustbox.draw(&state);
    }
    save_hosts(&state);
}

fn handle_event(event: rustbox::Event, state: &State) -> (bool, State) {
    let mut should_quit = false;
    // TODO(cgag): avoid all these default cases returning state somehow?
    let new_state = match event {
        rustbox::Event::KeyEvent(mkey) => {
            match mkey {
                Some(key) => match state.mode { 
                    // TODO(cgag): should support arrow keys as well
                    Mode::Normal => match key {
                        Key::Char('q') => { should_quit = true; state.clone() },
                        Key::Char('j') => { move_sel(state, Movement::Down)   },
                        Key::Char('k') => { move_sel(state, Movement::Up)     },
                        Key::Char('J') => { move_sel(state, Movement::Bottom) },
                        Key::Char('K') => { move_sel(state, Movement::Top)    }, 
                        Key::Char('i') => { insert_mode(state)  },
                        Key::Char('d') => { delete_selected(state)  },
                        Key::Char(' ') => { toggle_block(state) },
                        _  => { state.clone() }
                    },
                    Mode::Insert => match key {
                        Key::Enter => { 
                            let s = if state.adding.is_empty() {
                                state.clone()
                            } else {
                                add_url(&state, &state.adding)
                            };
                            normal_mode(&s) 
                        }
                        Key::Esc => { normal_mode(state) }, 
                        Key::Backspace => { backspace(state) },
                        Key::Char(c)   => { add_char(state, c) },
                        _ => { state.clone() }
                    }
                },
                _ => { state.clone()} // TODO(cgag): how could this branch ever be hit?
            }
        }
        _ => { state.clone() } 
    };

    (should_quit, new_state)
}

////////////////////////////
//  State manipulation   ///
////////////////////////////
fn move_sel(state: &State, movement: Movement) -> State {
    let mut new_state = State { selected: 0, 
                                adding:  state.adding.clone(),
                                domains: state.domains.clone(),
                                mode:    state.mode };
    match movement {
        Movement::Top => { new_state }
        Movement::Bottom => { 
            new_state.selected = state.domains.len();
            new_state
        }
        Movement::Up => {
            if state.selected == 0 {
                new_state.selected = state.domains.len() - 1;
            } else {
                new_state.selected =  state.selected - 1;
            }
            new_state
        }
        Movement::Down => {
            if state.selected == state.domains.len() - 1 {
                new_state.selected = 0;
            } else {
                new_state.selected = state.selected + 1;
            }
            new_state
        }
    }
}

fn normal_mode(state: &State) -> State {
    State { selected: state.selected
          , domains:  state.domains.clone()
          , adding:   state.adding.clone()
          , mode:     Mode::Normal }
}

fn insert_mode(state: &State) -> State {
    State { selected: state.selected
          , domains:  state.domains.clone()
          , adding:   state.adding.clone()
          , mode: Mode::Insert }
}

fn add_url(state: &State, url: &str) -> State {
    let d = Domain { url: String::from(url)
                   , status: DomainStatus::Blocked };

    let mut new_domains = state.domains.clone();
    new_domains.push(d);
    State { domains:  new_domains
          , selected: state.selected
          , adding:   String::from("")
          , mode:     state.mode.clone() }
}

fn delete_selected(state: &State) -> State {
    let mut new_domains = state.domains.clone();
    new_domains.remove(state.selected);

    State { domains:  new_domains
          , selected: state.selected
          , adding:   state.adding.clone()
          , mode:     state.mode.clone() }
}

fn add_char(state: &State, c: char) -> State {
    let mut new_adding = state.adding.clone();
    new_adding.push(c);

    State { domains: state.domains.clone()
          , selected: state.selected
          , adding: new_adding
          , mode: state.mode }
}

fn backspace(state: &State) -> State {
    let mut new_adding = state.adding.clone();

    match new_adding.pop() { 
        Some(_) => {},
        None => { new_adding = String::from("") },
    }

    State { domains: state.domains.clone()
          , selected: state.selected
          , adding: new_adding
          , mode: state.mode }
}


fn toggle_block(state: &State) -> State {
    let mut d = state.domains.iter().cloned().collect::<Vec<Domain>>();
    d[state.selected] = Domain { 
        url: d[state.selected].url.clone(),
        status: match d[state.selected].status {
            DomainStatus::Blocked   => DomainStatus::Unblocked,
            DomainStatus::Unblocked => DomainStatus::Blocked,
        },
    }; 

    State { selected: state.selected
          , domains:  d
          , adding:   state.adding.clone()
          , mode:     Mode::Normal }
}

/////////////////
// Persistence //
/////////////////
fn read_hosts() -> String {
    let mut hosts_file = match File::open("/etc/hosts") {
        Ok(file) => { file }
        Err(_) => { 
            panic!("Couldn't access hosts file, try running with sudo.") 
        }
    };

    // TODO(cgag): just return file handle so it's not all read into memory?
    // We just iterate over the lines atm.
    let mut s = String::new();
    hosts_file.read_to_string(&mut s).unwrap();
    s
}

fn parse_hosts(hosts_text: String) -> Vec<Domain> {
    let domain_lines = 
        hosts_text.lines()
            .take_while(|s| !s.starts_with("### End HostBlock"))
            .skip_while(|s| !s.starts_with("### HostBlock"))
            .skip(1) // drop the ### HostBlock line
            .map(|line| line.to_string())
            .collect::<Vec<String>>();

    domain_lines.iter()
        .map(|line| {
            let ip  = line.split_whitespace().nth(0).unwrap();
            let url = String::from(line.split_whitespace().nth(1).unwrap());
            Domain { 
                url: url,
                status: match UnicodeSegmentation::graphemes(ip, true).nth(0).unwrap() {
                    "#" => DomainStatus::Unblocked,
                    _   => DomainStatus::Blocked
                }
            }
        })
        .collect::<Vec<Domain>>()
}

fn save_hosts(state: &State) {
    fs::copy(Path::new("/etc/hosts"), Path::new("/etc/hosts.hb.back"))
        .ok()
        .expect("failed to backup hosts");

    // read again to decrease likelyhood of race condition?
    // Probalby a waste of time, maybe just hold original hosts
    // file in memory?
    let mut hosts_text = String::new();
    File::open("/etc/hosts").unwrap().read_to_string(&mut hosts_text).unwrap();

    // TODO(cgag): append the new hostblock list
    let before_block = 
        hosts_text
            .lines()
            .take_while(|s| !s.starts_with("### HostBlock"));

    let after_block =
        hosts_text
            .lines()
            .skip_while(|s| !s.starts_with("### End HostBlock"))
            .skip(1); // drop the ### End hostblock line

    let mut new_hosts = String::new();
    for line in before_block.chain(after_block) {
        new_hosts.push_str(line);
        new_hosts.push_str("\n");
    };

    new_hosts.push_str("### HostBlock\n");
    for domain in state.domains.iter() {
        let block_marker = match domain.status {
            DomainStatus::Blocked   => "",
            DomainStatus::Unblocked => "#"
        };
        new_hosts.push_str(block_marker);
        new_hosts.push_str("127.0.0.1\t");
        new_hosts.push_str(&domain.url);
        new_hosts.push_str("\n");
    };
    new_hosts.push_str("### End HostBlock\n");

    match File::create("/etc/hosts") {
        Ok(mut f) => match f.write_all(new_hosts.as_bytes()) {
            Ok(_)  => {},
            Err(e) => { panic!("Couldn't create new hosts file: {}", e); }
        },
        Err(e) => { println!("couldn't open hosts! {} ", e); panic!(e); }
    }
}


///////////////
// Rendering //
///////////////
// TODO(cgag): prefer &str?
fn render_domain(domain: &Domain) -> String { 
    let status_prefix = match domain.status {
        DomainStatus::Blocked => "[x] ",
        DomainStatus::Unblocked => "[ ] "
    };

    // TODO(cgag): better way to concat strings? 
    let mut s = String::from(status_prefix);
    s.extend(UnicodeSegmentation::graphemes(&*domain.url, true));
    s
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n { return String::from(s) }

    let tail = "...";
    let mut truncated = String::new();
    for grapheme in UnicodeSegmentation::graphemes(s, true).take(n - tail.len()) {
        truncated.push_str(grapheme)
    }
    truncated.push_str(tail);
    return truncated
}

fn last_n_chars(s: &str, n: usize) -> String {
    if s.len() <= n { return String::from(s) }

    let to_drop = s.len() - n;

    let mut res = String::new();
    for grapheme in UnicodeSegmentation::graphemes(s, true).skip(to_drop) {
        res.push_str(grapheme);
    }
    res
}

fn make_label(s: &str) -> String {
    let prefix_size = 5;

    let prefix = str_repeat(String::from(HORIZONTAL_LINE), prefix_size);
    let rest_of_line =
        str_repeat(String::from(HORIZONTAL_LINE), 
                   BOX_WIDTH - s.len() - prefix_size - 2);
                                    
    String::from(TOP_LEFT) + &prefix + s + &rest_of_line + TOP_RIGHT
}

fn str_repeat(s: String, n: usize) -> String {
    vec![String::from(s)].iter()
        .cloned()
        .cycle()
        .take(n)
        .collect::<Vec<String>>()
        .connect("")
}

fn make_bottom() -> String {
    let mut line = String::new();
    for _ in 0..(BOX_WIDTH - 2) {
        line.push_str(HORIZONTAL_LINE)
    }
    String::from(BOTTOM_LEFT) + &line + BOTTOM_RIGHT
}

// Can't just add methods to rustbox without introducing a trait or type 
// alias.
trait ScreenWriter {
    fn w(&self, x: usize, y: usize, text: &str);
    fn w_inv(&self, x: usize, y: usize, text: &str);
    fn draw(&self, state: &State);
}

impl ScreenWriter for RustBox {
    fn w(&self, x: usize, y: usize, text: &str) {
        self.print(x, y, rustbox::RB_BOLD, Color::White, Color::Black, text);
    }

    fn w_inv(&self, x: usize, y: usize, text: &str) {
        self.print(x, y, rustbox::RB_BOLD, Color::Black, Color::White, text);
    }

    fn draw(&self, state: &State) {
        self.clear();
        self.present();

        match state.mode {
            Mode::Normal => { 
                if state.domains.len() == 0 { 
                    self.w(0, 0, "No domains, hit i to enter insert mode");
                } else { 
                    self.w(0, 0, &make_label("Domains"));
                    for (i, domain) in state.domains.iter().enumerate() {
                        let y = i + 1;
                        let s = truncate(&render_domain(domain), 33);
                        self.w(0, y, VERTICAL_LINE);
                        if i == state.selected {
                            self.w_inv(2, y, &s);
                        } else {
                            self.w(2, y, &s);
                        }
                        self.w(BOX_WIDTH - 1, y, VERTICAL_LINE);
                    }
                    self.w(0, state.domains.len() + 1, &make_bottom());
                }
            },
            Mode::Insert => { 
                self.w(0, 0, &make_label("Add domain"));

                // TODO(cgag): method like w_bordered(...)
                self.w(0, 1, VERTICAL_LINE);
                self.w(2, 1, &last_n_chars(&state.adding, BOX_WIDTH - 5));
                self.w(min(state.adding.len() + 2, BOX_WIDTH - 3), 1, "_");
                self.w(BOX_WIDTH - 1, 1, VERTICAL_LINE);

                self.w(0, 2, VERTICAL_LINE);
                self.w(2, 2, "Press enter to finish.");
                self.w(BOX_WIDTH - 1, 2, VERTICAL_LINE);

                self.w(0, 3, &make_bottom());
            },
        }
        self.present();
    }
}
