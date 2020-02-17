#![cfg_attr(feature = "lints", feature(plugin))]
#![cfg_attr(feature = "lints", plugin(clippy))]

extern crate rustbox;
extern crate rand;
extern crate unicode_segmentation;
extern crate getopts;

use getopts::Options;
use std::env;

use std::cmp::min;
use std::default::Default;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use std::process::exit;

use rand::Rng;
use unicode_segmentation::UnicodeSegmentation;

use rustbox::{RustBox, Color, Key};

// TODO(cgag): remove as many unwraps as possible

// TODO(cgag): perhaps the rustbox instance should
// live in here, and then write/write_inverted, render, etc
// could be methods on State.
#[derive(Clone)]
struct State {
    selected: usize,
    domains: Vec<Domain>,
    adding: String,
    mode: Mode,
    status: Status,
    correct_pass: String,
    pass_input: String,
}


#[derive(Clone)]
struct Domain {
    url: String,
    status: DomainStatus,
}

#[derive(Clone)]
enum DomainStatus {
    Blocked,
    Unblocked,
}

#[derive(Clone)]
enum Status {
    Dirty,
    Clean,
}

enum Movement {
    Top,
    Bottom,
    Up,
    Down,
}

#[derive(Clone)]
enum Mode {
    Insert,
    Normal,
    Password,
    Help,
}

// taken straight from termui
static TOP_RIGHT: &'static str = "┐";
static VERTICAL_LINE: &'static str = "│";
static HORIZONTAL_LINE: &'static str = "─";
static TOP_LEFT: &'static str = "┌";
static BOTTOM_RIGHT: &'static str = "┘";
static BOTTOM_LEFT: &'static str = "└";

static BOX_WIDTH: usize = 55;

fn main() {
    match fs::copy(Path::new("/etc/hosts"), Path::new("/etc/hosts.hb.back")) {
        Ok(_) => (),
        Err(_) => {
            writeln!(&mut std::io::stderr(), "Couldn't access /etc/hosts.  Try running with sudo.")
                .unwrap();
            exit(1);
        }
    }

    let (show_menu, mut state) = read_args();


    if show_menu{
        let rustbox = RustBox::init(Default::default()).unwrap();
        rustbox.draw(&state);

        loop {
            if let rustbox::Event::KeyEvent(mkey) = rustbox.poll_event(false)
                                                           .ok()
                                                           .expect("poll failed") {
                match mkey {
                    Key::Ctrl('c') => break,
                    k => {
                        let (quit, new_state) = handle_key(k, &state);
                        if quit {
                            break
                        }
                        state = new_state;
                        rustbox.draw(&state);
                    }
                    //_ => {}
                }
            }
        }
    } // force rustbox out of scope to clear window, so that error messages
      // can be seen when saving.

    match save_hosts(&state) {
        Ok(_) => {}
        Err(e) => panic!(e),
    };
}
// true, the args allowed us to bypass the GUI
// false, nevermind show the GUI
fn read_args() -> (bool, State){
    fn print_usage(program: &str, opts: Options) {
        let brief = format!("Usage: {} [options]", program);
        print!("{}", opts.usage(&brief));
    }

    let state = State {
        selected: 0,
        domains: parse_hosts(read_hosts()),
        adding: String::from(""),
        pass_input: String::from(""),
        correct_pass: gen_pass(2),
        status: Status::Clean,
        mode: Mode::Normal,
    };

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("u", "unblock", "unblock all hosts (requires passphrase)");
    opts.optflag("b", "block", "block all hosts");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => {
            m
        }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return (false, state);
    }
    if matches.opt_present("b") {
        print!("hosts blocked");
        return (false, block_all(state));
    }
    if matches.opt_present("u"){
        // doing it like this doesn't change the runtime behavior to much
        if cfg!(feature = "commandline_unblock"){
            // fall into the menu to allow the passphrase
            return (true, unblock_all(state));
        }
        print!("unblock via commandline disabled in this build");
        return (false,state);
    }

    return (true, state);
}

fn handle_key(key: rustbox::Key, state: &State) -> (bool, State) {
    match state.mode {
        Mode::Normal => handle_normal_input(key, state),
        Mode::Insert => handle_insert_input(key, state),
        Mode::Password => handle_password_input(key, state),
        Mode::Help => handle_help_input(key, state),
    }
}

fn handle_normal_input(key: Key, state: &State) -> (bool, State) {
    let mut should_quit = false;

    // TODO(cgag): need to unify the return types of all these things
    // to avoid the nastiness seen in the 'q' branch.
    let new_state = match key {
        Key::Char('q') => {
            let (quit, new_state) = attempt_quit(state);
            should_quit = quit;
            new_state
        }
        Key::Esc => {
            let (quit, new_state) = attempt_quit(state);
            should_quit = quit;
            new_state
        }
        Key::Char('i') => insert_mode(state),
        Key::Char('h') => help_mode(state),
        Key::Char('j') | Key::Down => move_sel(state, Movement::Down),
        Key::Char('k') | Key::Up => move_sel(state, Movement::Up),
        Key::Char('J') => move_sel(state, Movement::Bottom),
        Key::Char('K') => move_sel(state, Movement::Top),
        Key::Char('d') => delete_selected(state),
        Key::Char(' ') => toggle_block(state),
        _ => state.clone(),
    };

    (should_quit, new_state)
}

fn attempt_quit(state: &State) -> (bool, State) {
    let mut should_quit = false;

    let new_state = match state.status {
        Status::Dirty => password_mode(&state),
        Status::Clean => {
            should_quit = true;
            state.clone()
        }
    };

    (should_quit, new_state)
}

fn handle_help_input(key: Key, state: &State) -> (bool, State) {
    let new_state = match key {
        Key::Esc | Key::Char('q') => normal_mode(state),
        Key::Char('i') => insert_mode(state),
        Key::Char('h') => help_mode(state),
        Key::Char(' ') => toggle_block(state),
        _ => state.clone(),
    };

    (false, new_state)
}

fn handle_insert_input(key: Key, state: &State) -> (bool, State) {
    let new_state = match key {
        Key::Enter => {
            let s = if state.adding.is_empty() {
                state.clone()
            } else {
                add_url(&state, &state.adding)
            };
            normal_mode(&s)
        }
        Key::Esc => normal_mode(state),
        Key::Backspace => backspace(state),
        Key::Char(c) => add_char(state, c),
        _ => state.clone(),
    };

    (false, new_state)
}

fn handle_password_input(key: Key, state: &State) -> (bool, State) {
    let mut should_quit = false;

    let new_state = match key {
        Key::Enter => {
            if state.pass_input == state.correct_pass {
                should_quit = true;
                state.clone()
            } else {
                let mut new_state = state.clone();
                new_state.pass_input = String::from("");
                new_state
            }
        }
        Key::Esc => normal_mode(state),
        Key::Backspace => password_backspace(state),
        Key::Char(c) => add_password_char(state, c),
        _ => state.clone(),
    };

    (should_quit, new_state)
}

////////////////////////////
//  State manipulation   ///
////////////////////////////
fn move_sel(state: &State, movement: Movement) -> State {
    let mut new_state = state.clone();

    new_state.selected = match movement {
        Movement::Top => 0,
        Movement::Bottom => state.domains.len() - 1,
        Movement::Up => {
            if state.selected == 0 {
                state.domains.len() - 1
            } else {
                state.selected - 1
            }
        }
        Movement::Down => {
            if state.selected == state.domains.len() - 1 {
                0
            } else {
                state.selected + 1
            }
        }
    };

    new_state
}

fn switch_mode(state: &State, mode: Mode) -> State {
    let mut new_state = state.clone();
    new_state.mode = mode;
    new_state
}
// TODO(cgag): just clone the state and mutate the individual field.
fn normal_mode(state: &State) -> State {
    switch_mode(state, Mode::Normal)
}

fn password_mode(state: &State) -> State {
    switch_mode(state, Mode::Password)
}

fn insert_mode(state: &State) -> State {
    switch_mode(state, Mode::Insert)
}

fn help_mode(state: &State) -> State {
    switch_mode(state, Mode::Help)
}

fn add_url(state: &State, url: &str) -> State {
    let mut new_state = state.clone();

    new_state.domains.push(Domain {
        url: String::from(url),
        status: DomainStatus::Blocked,
    });
    new_state.adding = "".to_owned();

    new_state
}

fn delete_selected(state: &State) -> State {
    let mut new_state = state.clone();
    new_state.domains.remove(state.selected);
    new_state.selected = if state.selected > 0 {
        state.selected - 1
    } else {
        0
    };
    new_state.status = Status::Dirty;
    new_state
}

fn add_char(state: &State, c: char) -> State {
    let mut new_state = state.clone();
    new_state.adding.push(c);
    new_state
}

// TODO(cgag): these redundant fns (this and backspace) are smelly
fn add_password_char(state: &State, c: char) -> State {
    let mut new_state = state.clone();
    new_state.pass_input.push(c);
    new_state
}

fn backspace(state: &State) -> State {
    let mut new_state = state.clone();
    new_state.adding.pop();
    new_state
}

fn password_backspace(state: &State) -> State {
    let mut new_state = state.clone();
    new_state.pass_input.pop();
    new_state
}

fn block_all(state:State) -> State{
    let mut new_state = state.clone();
    new_state.domains = new_state.domains.into_iter().map(|domain| Domain{
        url:domain.url.clone(),
        status:DomainStatus::Blocked
    }).collect();
    new_state
}

fn unblock_all(state:State) -> State{
    let mut new_state = state.clone();
    new_state.domains = new_state.domains.into_iter().map(|domain| Domain{
        url:domain.url.clone(),
        status:DomainStatus::Unblocked
    }).collect();
    new_state.status = Status::Dirty;
    new_state.correct_pass = gen_pass(6);
    new_state.mode = Mode::Password;
    new_state
}

fn toggle_block(state: &State) -> State {
    let mut new_state = state.clone();
    let mut dirty = false;

    let mut d = state.domains.clone();
    d[state.selected] = Domain {
        url: d[state.selected].url.clone(),
        status: match d[state.selected].status {
            DomainStatus::Blocked => {
                dirty = true;
                DomainStatus::Unblocked
            }
            DomainStatus::Unblocked => DomainStatus::Blocked,
        },
    };

    new_state.domains = d;
    if dirty {
        new_state.status = Status::Dirty;
    }

    new_state
}

/////////////////
// Persistence //
/////////////////
fn read_hosts() -> String {
    let mut hosts_file = match File::open("/etc/hosts") {
        Ok(file) => file,
        Err(_) => panic!("Couldn't access hosts file, try running with sudo."),
    };

    // TODO(cgag): just return file handle so it's not all read into memory?
    // We just iterate over the lines atm.
    let mut s = String::new();
    match hosts_file.read_to_string(&mut s) {
        Ok(_) => {}
        Err(e) => panic!("Couldn't read hosts file: {}", e),
    }
    s
}

fn parse_hosts(hosts_text: String) -> Vec<Domain> {
    hosts_text.lines()
        .take_while(|s| !s.starts_with("### End HostBlock"))
        .skip_while(|s| !s.starts_with("### HostBlock"))
        .skip(1)
        .map(|line| {
            let ip = match line.split_whitespace().nth(0) {
                Some(ip) => ip,
                None => panic!("Failed to parse a valid IP from line: {}", line),
            };
            let url = match line.split_whitespace().nth(1) {
                Some(url) => String::from(url),
                None => panic!("Failed to parse a valid URL from line: {}", line),
            };

            Domain {
                url: url,
                status: match UnicodeSegmentation::graphemes(ip, true)
                                  .nth(0)
                                  .unwrap() {
                    "#" => DomainStatus::Unblocked,
                    _ => DomainStatus::Blocked,
                },
            }
        })
        .collect::<Vec<Domain>>()
}

fn save_hosts(state: &State) -> Result<(), io::Error> {
    let mut hosts_file = try!(File::open("/etc/hosts"));
    let mut hosts_text = String::new();
    try!(hosts_file.read_to_string(&mut hosts_text));

    let before_block = hosts_text.lines()
                                 .take_while(|s| !s.starts_with("### HostBlock"));

    let after_block = hosts_text.lines()
                                .skip_while(|s| !s.starts_with("### End HostBlock"))
                                .skip(1); // drop the ### End hostblock line

    let mut new_hosts = String::new();
    for line in before_block.chain(after_block) {
        new_hosts.push_str(line);
        new_hosts.push_str("\n");
    };

    new_hosts.push_str("### HostBlock\n");
    for domain in &state.domains {
        let block_marker = match domain.status {
            DomainStatus::Blocked => "",
            DomainStatus::Unblocked => "#",
        };
        new_hosts.push_str(block_marker);
        new_hosts.push_str("127.0.0.1\t");
        new_hosts.push_str(&domain.url);
        new_hosts.push_str("\n");
    };
    new_hosts.push_str("### End HostBlock\n");

    let mut file = try!(File::create("/etc/hosts"));
    try!(file.write_all(new_hosts.as_bytes()));
    Ok(())
}


///////////////
// Rendering //
///////////////
fn render_domain(domain: &Domain) -> String {
    let status_prefix = match domain.status {
        DomainStatus::Blocked => "[x] ",
        DomainStatus::Unblocked => "[ ] ",
    };

    String::from(status_prefix) + &domain.url
}

fn make_label(s: &str) -> String {
    let prefix_size = 1;

    let prefix = str_repeat(String::from(HORIZONTAL_LINE), prefix_size);
    let rest_of_line = str_repeat(String::from(HORIZONTAL_LINE),
                                  BOX_WIDTH - s.len() - prefix_size - 2);

    String::from(TOP_LEFT) + &prefix + s + &rest_of_line + TOP_RIGHT
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
    fn w_boxed(&self, x: usize, y: usize, text: &str);
    fn draw(&self, state: &State);
}

// TODO(cgag): should render state like selecth, into a list of things
// to write, e.g, [(Boxed, Inverted, "hello")]
impl ScreenWriter for RustBox {
    fn w(&self, x: usize, y: usize, text: &str) {
        self.print(x, y, rustbox::RB_BOLD, Color::White, Color::Black, text);
    }

    fn w_inv(&self, x: usize, y: usize, text: &str) {
        self.print(x, y, rustbox::RB_BOLD, Color::Black, Color::White, text);
    }

    fn w_boxed(&self, x: usize, y: usize, text: &str) {
        self.w(x, y, VERTICAL_LINE);
        self.w(x + 2, y, text);
        self.w(x + BOX_WIDTH - 1, y, VERTICAL_LINE);
    }

    fn draw(&self, state: &State) {
        self.clear();
        self.present();

        match state.mode {
            Mode::Normal => {
                if state.domains.is_empty() {
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
            }
            Mode::Insert => {
                self.w(0, 0, &make_label("Add domain"));

                // TODO(cgag): method like w_bordered(...)
                self.w(0, 1, VERTICAL_LINE);
                self.w(2, 1, &last_n_chars(&state.adding, BOX_WIDTH - 5));
                self.w(min(state.adding.len() + 2, BOX_WIDTH - 3), 1, "_");
                self.w(BOX_WIDTH - 1, 1, VERTICAL_LINE);

                self.w_boxed(0, 2, "Press enter to finish.");

                self.w(0, 3, &make_bottom());
            }
            Mode::Password => {
                // TODO(cgag): like 95% duplication from the Mode::Insert
                // arm...
                self.w(0, 0, &make_label("Type the passphrase below to save"));
                self.w_boxed(0, 1, &state.correct_pass);

                self.w(0, 2, VERTICAL_LINE);
                self.w(2, 2, &last_n_chars(&state.pass_input, BOX_WIDTH - 5));
                self.w(min(state.pass_input.len() + 2, BOX_WIDTH - 3), 2, "_");
                self.w(BOX_WIDTH - 1, 2, VERTICAL_LINE);

                self.w(0, 3, &make_bottom());
            }
            Mode::Help => {
                let mut y = 0;
                self.w(0, y, &make_label("Help"));
                y += 1;

                let movements = vec![("j", "down"),
                                     ("k", "up"),
                                     ("J", "GOTO bottom"),
                                     ("K", "GOTO top")];

                for &(movement, desc) in &movements {
                    self.w_boxed(0, y, &(String::from(movement) + " - " + desc));
                    y += 1;
                }

                self.w_boxed(0, y, "");
                y += 1;

                let controls = vec![("i", "Add a domain to block."),
                                    ("d", "Remove highlighted domain."),
                                    ("<space>", "Toggle blocked/unblocked"),
                                    ("q", "quit / back one screen")];
                for &(control, desc) in &controls {
                    self.w_boxed(0, y, &(String::from(control) + " - " + desc));
                    y += 1;
                }

                self.w(0, controls.len() + 1 + movements.len() + 1, &make_bottom());
            }
        }
        self.present();
    }
}

fn gen_pass(num_words: usize) -> String {
    let mut choices = vec!["dilate",
                           "indict",
                           "sacrilegious",
                           "ingenious",
                           "minuscule",
                           "onomatopoeia",
                           "accommodate",
                           "conscientious",
                           "Wednesday",
                           "acquiesce",
                           "bologna",
                           "fuchsia",
                           "nauseous",
                           "orangutan",
                           "paraphernalia",
    ];

    let mut rng = rand::thread_rng();
    rng.shuffle(&mut choices);

    choices.into_iter()
           .take(num_words)
           .map(|choice| choice.to_owned())
           .collect::<Vec<String>>()
           .join(" ")
}

fn str_repeat(s: String, n: usize) -> String {
    std::iter::repeat(s).take(n).collect::<Vec<_>>().join("")
}


fn truncate(s: &str, n: usize) -> String {
    let tail = "...";

    if s.len() <= n {
        return String::from(s)
    }

    UnicodeSegmentation::graphemes(s, true).take(n - tail.len()).collect::<String>() + tail
}

fn last_n_chars(s: &str, n: usize) -> String {
    if s.len() <= n {
        return String::from(s)
    }

    UnicodeSegmentation::graphemes(s, true).skip(s.len() - n).collect::<String>()
}
