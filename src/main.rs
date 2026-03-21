use crossterm::{
    cursor::{self, MoveUp},
    event::{self, Event, KeyCode},
    execute,
    style::{self, Stylize},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen, ScrollDown, ScrollUp},
};
use std::{
    env, fs,
    io::{self, Write},
    os::unix::thread,
    path::PathBuf,
    thread::*,
    time::Duration,
};

#[derive(Debug, Clone)]
struct State {
    current_dir: PathBuf,
    directories: Vec<String>,
    selected_dir: Option<usize>,
}

impl State {
    fn new() -> Self {
        let current_dir = env::current_dir().expect("Couldn't get current directory");
        let directories = get_dirs(&current_dir);
        let selected_dir = (!directories.is_empty()).then_some(0);

        State {
            current_dir: current_dir,
            directories: directories,
            selected_dir: selected_dir,
        }
    }
}

fn test() -> anyhow::Result<()> {
    let mut stdout = io::stdout();

    sleep(Duration::from_secs(2));
    execute!(stdout, style::Print("text \n"))?;
    sleep(Duration::from_secs(2));
    execute!(stdout, cursor::MoveUp(1))?;
    sleep(Duration::from_secs(2));

    // execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
    // sleep(Duration::from_secs(2));

    execute!(
        stdout,
        terminal::Clear(ClearType::CurrentLine),
        cursor::MoveToColumn(0)
    )?;
    print!("\x1B[M"); // ANSI "delete line" escape — removes line, shifts lines below up
    stdout.flush()?;

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => {
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // test()?;
    // return Ok(());

    // // Check if program was run from a terminal (Adds unecessary bloat to program)
    // if !std::io::stdout().is_terminal() {
    //     notify_rust::Notification::new()
    //         .summary("Folder Navigator")
    //         .body("This program must be run from a terminal.\nOpen a terminal and run: folder-navigator")
    //         .icon("dialog-information")
    //         .show()
    //         .ok(); // ignore if notification daemon isn't available
    //     return Ok(());
    // }

    // Set up navigation
    let mut navigator_state = State::new();

    // Get stdout
    let mut stdout = io::stdout();

    // Enable raw mode to prevent users key inputs from being printed
    terminal::enable_raw_mode()?;

    // Enter alternate screen (If we don't there are a lot of issues with directories containing a lot of segments)
    execute!(stdout, EnterAlternateScreen)?;
    let mut rendered_count: usize = 0;

    loop {
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
        )?;
        // Get name of current directory
        let dir_name = navigator_state
            .current_dir
            .file_name()
            .unwrap_or(navigator_state.current_dir.as_os_str()) // fallback for root "/"
            .to_string_lossy();

        // Print name of current directory in green
        execute!(
            stdout,
            style::Print(format!("{}\r\n", dir_name).green().bold())
        )?;

        //TODO only render directories that fit
        let (_, terminal_height) = terminal::size()?;

        let max_directories_that_fit = (terminal_height as usize).saturating_sub(2);

        // Print Avalible directories
        for (i, entry) in navigator_state.directories.iter().enumerate() {
            if navigator_state.selected_dir == Some(i) {
                execute!(
                    stdout,
                    style::Print(format!("  > {}\r\n", entry).cyan().bold())
                )?;
            } else {
                execute!(stdout, style::Print(format!("    {}\r\n", entry)))?;
            }
        }

        execute!(
            stdout,
            ScrollDown(
                ((navigator_state.directories.len() as u16).saturating_sub(terminal_height)) as u16
            )
        )?;

        // execute!(stdout, ScrollDown(1))?;

        rendered_count = navigator_state.directories.len(); // snapshot before any mutation

        // Wait for user input
        if let Event::Key(key) = event::read()? {
            // Match to what key user pressed
            match key.code {
                KeyCode::Up => {
                    navigator_state.selected_dir = Some(
                        (navigator_state.selected_dir.unwrap_or(0)
                            + navigator_state.directories.len()
                            - 1)
                            % navigator_state.directories.len(),
                    );
                }
                KeyCode::Down => {
                    navigator_state.selected_dir = Some(
                        (navigator_state.selected_dir.unwrap_or(0)
                            + navigator_state.directories.len()
                            + 1)
                            % navigator_state.directories.len(),
                    );
                }
                KeyCode::Left => {
                    // Clear directories from terminal
                    // clear_terminal_directories(&mut stdout, &state)?;
                    // Move up one directory
                    navigator_state.current_dir.pop();
                    // Get the new directories
                    navigator_state.directories = get_dirs(&navigator_state.current_dir);
                    // Reset selected directory
                    navigator_state.selected_dir = Some(0);
                }
                KeyCode::Right => {
                    // Check if a directory is selected
                    if let Some(selected_dir) = navigator_state.selected_dir {
                        // Get selected dir name
                        let dir = navigator_state
                            .directories
                            .get(selected_dir)
                            .expect("Couldn't get name of selected directory");

                        // Move in to the directory
                        navigator_state.current_dir.push(dir);
                        // Get the new directories
                        navigator_state.directories = get_dirs(&navigator_state.current_dir);
                        // Reset selected directory if directories exist
                        navigator_state.selected_dir =
                            (!navigator_state.directories.is_empty()).then_some(0);
                    } else {
                        // If no directory selected check if there are any, and if so select first one
                        navigator_state.selected_dir =
                            (!navigator_state.directories.is_empty()).then_some(0);
                    }
                }
                KeyCode::Esc => {
                    break;
                }
                _ => {}
            }
        }
    }

    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    
    terminal::disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;
    Ok(())
}

fn clear_terminal_directories(stdout: &mut io::Stdout, state: &State) -> anyhow::Result<()> {
    execute!(*stdout, cursor::MoveUp((state.directories.len()) as u16))?;
    execute!(*stdout, terminal::Clear(ClearType::FromCursorDown))?;
    Ok(())
}

fn clear_terminal_lines(stdout: &mut io::Stdout, rendered_count: usize) -> anyhow::Result<()> {
    if rendered_count > 0 {
        execute!(*stdout, cursor::MoveUp((rendered_count) as u16))?;
        execute!(*stdout, terminal::Clear(ClearType::FromCursorDown))?;
    }
    Ok(())
}

fn get_dirs(path: &PathBuf) -> Vec<String> {
    let mut dirs: Vec<String> = fs::read_dir(path)
        // Panic if couldn't read directory
        .unwrap()
        // Filter out entries that can not be read
        .filter_map(|res| res.ok())
        // Filter out entries that are not directories
        .filter(|e| e.path().is_dir())
        // Get the names of the directories
        .map(|e| e.file_name().into_string().unwrap())
        // Collect iterator as vec<String>
        .collect();
    // Sort alphanumerically and move directories starting with . to end
    dirs.sort_unstable_by_key(|name| (name.starts_with('.'), name.clone()));
    dirs
}
