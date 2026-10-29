//! The command line, parsed by hand: one command and two flags.

pub const USAGE: &str = "\
Usage: siderealctl <command> [options]

Commands:
  reset-admin-password  Create the administrator, or set a new password
                        and end every session

Options:
  --username <name>     The administrator's username. Required with
                        --password-stdin before the instance is claimed
  --password-stdin      Read the password from the first line of stdin
                        instead of prompting
  --help                Print this help
";

#[derive(Debug, PartialEq, Eq)]
pub enum Parsed {
    Help,
    ResetAdminPassword(Options),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub username: Option<String>,
    pub password_stdin: bool,
}

/// A command line `siderealctl` does not accept, with what is wrong with it.
#[derive(Debug, PartialEq, Eq)]
pub struct Usage(pub String);

/// Parse the arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Parsed, Usage> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        None => Err(Usage("no command given".into())),
        Some("--help") => Ok(Parsed::Help),
        Some("reset-admin-password") => options(args),
        Some(flag) if flag.starts_with('-') => Err(Usage(format!("unknown option {flag}"))),
        Some(command) => Err(Usage(format!("unknown command {command}"))),
    }
}

fn options(mut args: impl Iterator<Item = String>) -> Result<Parsed, Usage> {
    let mut options = Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" => return Ok(Parsed::Help),
            "--password-stdin" => options.password_stdin = true,
            "--username" => {
                let value = args
                    .next()
                    .ok_or_else(|| Usage("--username needs a value".into()))?;
                if options.username.replace(value).is_some() {
                    return Err(Usage("--username given twice".into()));
                }
            }
            _ => return Err(Usage(format!("unknown option {arg}"))),
        }
    }
    Ok(Parsed::ResetAdminPassword(options))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Parsed, Usage> {
        super::parse(args.iter().map(|arg| arg.to_string()))
    }

    fn usage(args: &[&str]) -> String {
        match parse(args) {
            Err(Usage(message)) => message,
            other => panic!("expected a usage error for {args:?}, got {other:?}"),
        }
    }

    #[test]
    fn help_before_a_command_is_help() {
        assert_eq!(parse(&["--help"]), Ok(Parsed::Help));
    }

    #[test]
    fn help_after_the_command_is_help() {
        assert_eq!(parse(&["reset-admin-password", "--help"]), Ok(Parsed::Help));
    }

    #[test]
    fn the_command_alone_prompts_for_everything() {
        assert_eq!(
            parse(&["reset-admin-password"]),
            Ok(Parsed::ResetAdminPassword(Options::default()))
        );
    }

    #[test]
    fn the_command_carries_its_flags() {
        assert_eq!(
            parse(&[
                "reset-admin-password",
                "--password-stdin",
                "--username",
                "sam"
            ]),
            Ok(Parsed::ResetAdminPassword(Options {
                username: Some("sam".into()),
                password_stdin: true,
            }))
        );
    }

    #[test]
    fn an_empty_username_parses_as_an_empty_string() {
        assert_eq!(
            parse(&["reset-admin-password", "--username", ""]),
            Ok(Parsed::ResetAdminPassword(Options {
                username: Some(String::new()),
                password_stdin: false,
            }))
        );
    }

    #[test]
    fn no_command_is_a_usage_error() {
        assert_eq!(usage(&[]), "no command given");
    }

    #[test]
    fn an_unknown_command_is_a_usage_error() {
        assert_eq!(usage(&["rotate-keys"]), "unknown command rotate-keys");
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        assert_eq!(
            usage(&["reset-admin-password", "--force"]),
            "unknown option --force"
        );
    }

    #[test]
    fn a_flag_before_the_command_is_a_usage_error() {
        assert_eq!(
            usage(&["--password-stdin", "reset-admin-password"]),
            "unknown option --password-stdin"
        );
    }

    #[test]
    fn a_username_flag_without_a_value_is_a_usage_error() {
        assert_eq!(
            usage(&["reset-admin-password", "--username"]),
            "--username needs a value"
        );
    }

    #[test]
    fn a_username_given_twice_is_a_usage_error() {
        assert_eq!(
            usage(&[
                "reset-admin-password",
                "--username",
                "sam",
                "--username",
                "alex"
            ]),
            "--username given twice"
        );
    }

    #[test]
    fn the_usage_names_the_command_and_its_flags() {
        for name in ["reset-admin-password", "--username", "--password-stdin"] {
            assert!(USAGE.contains(name), "{name}");
        }
    }
}
