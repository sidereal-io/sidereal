//! The questions the command asks, in order, and the checks between them.
//! Nothing here touches the database, so a refusal leaves no change.
use super::{args::Options, Terminal};
use sidereal_core::access::{check_password, check_username, generate_password, CredentialRule};
use std::io;

/// Whether an administrator exists, read before anything is asked.
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Create,
    Reset { administrator: String },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    /// The operator left the prompt empty and the command chose the password.
    pub generated: bool,
}

/// Why the command stops before writing. The message never holds a password.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The command line cannot work in this state: exit 2.
    Usage(String),
    /// Exit 1.
    Refused(String),
}

pub fn credentials(
    state: &State,
    options: &Options,
    terminal: &mut impl Terminal,
) -> Result<Credentials, Refusal> {
    let username = username(state, options, terminal)?;
    if options.password_stdin {
        let password = answer(terminal.read_line())?
            .filter(|password| !password.is_empty())
            .ok_or_else(|| refused("a password is required on stdin"))?;
        check_password(&password).map_err(rule)?;
        return Ok(Credentials {
            username,
            password,
            generated: false,
        });
    }
    let password = hidden(terminal, "New password: ")?;
    if password.is_empty() {
        if !terminal.is_interactive() {
            return Err(refused("a password is required"));
        }
        return Ok(Credentials {
            username,
            password: generate_password(),
            generated: true,
        });
    }
    check_password(&password).map_err(rule)?;
    if hidden(terminal, "Repeat password: ")? != password {
        return Err(refused("passwords do not match"));
    }
    Ok(Credentials {
        username,
        password,
        generated: false,
    })
}

fn username(
    state: &State,
    options: &Options,
    terminal: &mut impl Terminal,
) -> Result<String, Refusal> {
    match (state, &options.username) {
        (State::Reset { administrator }, Some(given)) if given != administrator => Err(refused(
            &format!("--username must name the administrator, {administrator}"),
        )),
        (State::Reset { administrator }, _) => Ok(administrator.clone()),
        (State::Create, given) => {
            let username = match given {
                Some(given) => given.clone(),
                None if options.password_stdin => return Err(Refusal::Usage(
                    "--username is required with --password-stdin while no administrator exists"
                        .into(),
                )),
                None => {
                    prompt(terminal, "Username: ")?;
                    answer(terminal.read_line())?.ok_or_else(closed)?
                }
            };
            check_username(&username).map_err(rule)?;
            Ok(username)
        }
    }
}

fn hidden(terminal: &mut impl Terminal, text: &str) -> Result<String, Refusal> {
    prompt(terminal, text)?;
    answer(terminal.read_hidden())?.ok_or_else(closed)
}

fn prompt(terminal: &mut impl Terminal, text: &str) -> Result<(), Refusal> {
    terminal
        .prompt(text)
        .map_err(|error| refused(&error.to_string()))
}

fn answer(read: io::Result<Option<String>>) -> Result<Option<String>, Refusal> {
    read.map_err(|error| refused(&error.to_string()))
}

fn refused(message: &str) -> Refusal {
    Refusal::Refused(message.into())
}

fn closed() -> Refusal {
    refused("stdin closed before an answer")
}

fn rule(rule: CredentialRule) -> Refusal {
    refused(&rule.to_string())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Answers each read from a script; an exhausted script is a closed stdin.
    pub(crate) struct Scripted {
        pub interactive: bool,
        pub answers: VecDeque<&'static str>,
        pub prompts: Vec<String>,
    }

    impl Scripted {
        pub fn new(interactive: bool, answers: &[&'static str]) -> Self {
            Self {
                interactive,
                answers: answers.iter().copied().collect(),
                prompts: Vec::new(),
            }
        }
    }

    impl Terminal for Scripted {
        fn is_interactive(&self) -> bool {
            self.interactive
        }
        fn prompt(&mut self, text: &str) -> io::Result<()> {
            self.prompts.push(text.to_owned());
            Ok(())
        }
        fn read_line(&mut self) -> io::Result<Option<String>> {
            Ok(self.answers.pop_front().map(str::to_owned))
        }
        fn read_hidden(&mut self) -> io::Result<Option<String>> {
            self.read_line()
        }
    }

    const PASSWORD: &str = "correct-horse-battery";

    fn create() -> State {
        State::Create
    }

    fn reset() -> State {
        State::Reset {
            administrator: "sam".into(),
        }
    }

    fn flags(username: Option<&str>, password_stdin: bool) -> Options {
        Options {
            username: username.map(str::to_owned),
            password_stdin,
        }
    }

    fn typed(username: &str, password: &str) -> Credentials {
        Credentials {
            username: username.into(),
            password: password.into(),
            generated: false,
        }
    }

    fn refused(message: &str) -> Result<Credentials, Refusal> {
        Err(Refusal::Refused(message.into()))
    }

    #[test]
    fn the_create_state_asks_for_a_username_then_the_password_twice() {
        let mut terminal = Scripted::new(true, &["sam", PASSWORD, PASSWORD]);
        assert_eq!(
            credentials(&create(), &flags(None, false), &mut terminal),
            Ok(typed("sam", PASSWORD))
        );
        assert_eq!(
            terminal.prompts,
            ["Username: ", "New password: ", "Repeat password: "]
        );
    }

    #[test]
    fn a_username_flag_skips_the_username_prompt() {
        let mut terminal = Scripted::new(true, &[PASSWORD, PASSWORD]);
        assert_eq!(
            credentials(&create(), &flags(Some("sam"), false), &mut terminal),
            Ok(typed("sam", PASSWORD))
        );
        assert_eq!(terminal.prompts, ["New password: ", "Repeat password: "]);
    }

    #[test]
    fn the_reset_state_asks_only_for_the_password_twice() {
        let mut terminal = Scripted::new(true, &[PASSWORD, PASSWORD]);
        assert_eq!(
            credentials(&reset(), &flags(None, false), &mut terminal),
            Ok(typed("sam", PASSWORD))
        );
        assert_eq!(terminal.prompts, ["New password: ", "Repeat password: "]);
    }

    #[test]
    fn a_short_password_is_refused_before_the_repeat() {
        let mut terminal = Scripted::new(true, &["short", "short"]);
        assert_eq!(
            credentials(&reset(), &flags(None, false), &mut terminal),
            refused("password must be 8 to 256 bytes")
        );
        assert_eq!(terminal.prompts, ["New password: "]);
    }

    #[test]
    fn a_padded_username_is_refused_before_the_password() {
        let mut terminal = Scripted::new(true, &[" sam", PASSWORD, PASSWORD]);
        assert_eq!(
            credentials(&create(), &flags(None, false), &mut terminal),
            refused("username must not start or end with whitespace")
        );
        assert_eq!(terminal.prompts, ["Username: "]);
    }

    #[test]
    fn an_empty_username_flag_is_refused() {
        let mut terminal = Scripted::new(true, &[PASSWORD]);
        assert_eq!(
            credentials(&create(), &flags(Some(""), true), &mut terminal),
            refused("username must be 1 to 64 characters")
        );
    }

    #[test]
    fn a_repeat_that_differs_is_refused() {
        let mut terminal = Scripted::new(true, &[PASSWORD, "correct-horse-batery"]);
        assert_eq!(
            credentials(&reset(), &flags(None, false), &mut terminal),
            refused("passwords do not match")
        );
    }

    #[test]
    fn an_empty_answer_on_a_terminal_generates_a_password_without_the_repeat() {
        let mut terminal = Scripted::new(true, &[""]);
        let answer = credentials(&reset(), &flags(None, false), &mut terminal).unwrap();
        assert!(answer.generated);
        assert_eq!(answer.password.len(), 24);
        assert_eq!(terminal.prompts, ["New password: "]);
    }

    #[test]
    fn an_empty_answer_through_a_pipe_is_refused() {
        let mut terminal = Scripted::new(false, &[""]);
        assert_eq!(
            credentials(&reset(), &flags(None, false), &mut terminal),
            refused("a password is required")
        );
    }

    #[test]
    fn a_stdin_closed_at_a_prompt_is_refused() {
        for answers in [&[][..], &[PASSWORD][..]] {
            let mut terminal = Scripted::new(true, answers);
            assert_eq!(
                credentials(&reset(), &flags(None, false), &mut terminal),
                refused("stdin closed before an answer"),
                "{answers:?}"
            );
        }
    }

    #[test]
    fn a_stdin_closed_at_the_username_prompt_is_refused() {
        let mut terminal = Scripted::new(false, &[]);
        assert_eq!(
            credentials(&create(), &flags(None, false), &mut terminal),
            refused("stdin closed before an answer")
        );
    }

    #[test]
    fn password_stdin_reads_one_line_without_prompting() {
        let mut terminal = Scripted::new(false, &[PASSWORD]);
        assert_eq!(
            credentials(&reset(), &flags(None, true), &mut terminal),
            Ok(typed("sam", PASSWORD))
        );
        assert!(terminal.prompts.is_empty());
    }

    #[test]
    fn password_stdin_refuses_an_empty_line_even_on_a_terminal() {
        let mut terminal = Scripted::new(true, &[""]);
        assert_eq!(
            credentials(&reset(), &flags(None, true), &mut terminal),
            refused("a password is required on stdin")
        );
    }

    #[test]
    fn password_stdin_refuses_a_closed_stdin() {
        let mut terminal = Scripted::new(false, &[]);
        assert_eq!(
            credentials(&reset(), &flags(None, true), &mut terminal),
            refused("a password is required on stdin")
        );
    }

    #[test]
    fn password_stdin_in_the_create_state_needs_a_username() {
        let mut terminal = Scripted::new(false, &[PASSWORD]);
        let Err(Refusal::Usage(message)) =
            credentials(&create(), &flags(None, true), &mut terminal)
        else {
            panic!("expected a usage error");
        };
        assert!(message.contains("--username"), "{message}");
    }

    #[test]
    fn a_username_that_is_not_the_administrator_is_refused() {
        let mut terminal = Scripted::new(false, &[PASSWORD]);
        assert_eq!(
            credentials(&reset(), &flags(Some("alex"), true), &mut terminal),
            refused("--username must name the administrator, sam")
        );
    }

    #[test]
    fn a_username_that_is_the_administrator_is_accepted() {
        let mut terminal = Scripted::new(false, &[PASSWORD]);
        assert_eq!(
            credentials(&reset(), &flags(Some("sam"), true), &mut terminal),
            Ok(typed("sam", PASSWORD))
        );
    }
}
