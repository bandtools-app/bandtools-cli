use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::CommandFactory;
use clap_complete::generate;
use reqwest::Method;
use serde_json::{Value, json};

use crate::{
    api::{ApiClient, Body, QueryParams, ensure_no_body, json_from_data, response_json},
    cli::*,
    config::{self, ConfigOverrides, FileConfig},
    output::{OutputFormat, print_value},
};

pub fn run(cli: Cli) -> Result<()> {
    let Cli {
        api_token,
        api_url,
        config: config_path,
        json,
        compact_json,
        plain,
        no_colour,
        command,
    } = cli;

    let format = if compact_json {
        OutputFormat::Json { pretty: false }
    } else if json {
        OutputFormat::Json { pretty: true }
    } else if plain {
        OutputFormat::Plain
    } else {
        OutputFormat::Tui { colour: !no_colour }
    };

    match command {
        Command::Config(command) => run_config(command, config_path, format),
        Command::Completions(command) => {
            let mut clap_command = Cli::command();
            generate(
                command.shell,
                &mut clap_command,
                "bt",
                &mut std::io::stdout(),
            );
            Ok(())
        }
        command => {
            let config = config::resolve(ConfigOverrides {
                api_token,
                api_url,
                config_path,
            })?;
            let client = ApiClient::new(&config)?;

            match command {
                Command::Subscribers(command) => subscribers(&client, format, command),
                Command::Account(command) => account(&client, format, command),
                Command::Newsletters(command) => newsletters(&client, format, command),
                Command::SharedNewsletters(command) => shared_newsletters(&client, format, command),
                Command::AutomaticNewsletters(command) => {
                    automatic_newsletters(&client, format, command)
                }
                Command::Config(_) | Command::Completions(_) => unreachable!("handled above"),
            }
        }
    }
}

fn run_config(
    command: ConfigCommand,
    cli_path: Option<PathBuf>,
    format: OutputFormat,
) -> Result<()> {
    let path = config::config_path(cli_path)?;
    let mut file = config::load(&path)?;

    match command.command {
        ConfigSubcommand::Path => {
            println!("{}", path.display());
        }
        ConfigSubcommand::Show => {
            let value = json!({
                "config_path": path,
                "api_token": file.api_token.as_ref().map(|_| "<configured>"),
                "api_url": file.api_url,
            });
            print_value(format, &value)?;
        }
        ConfigSubcommand::SetToken { api_token } => {
            file.api_token = Some(api_token);
            config::save(&path, &file)?;
            println!("Updated {}", path.display());
        }
        ConfigSubcommand::SetApiUrl { api_url } => {
            file.api_url = Some(config::normalise_api_url(&api_url)?);
            config::save(&path, &file)?;
            println!("Updated {}", path.display());
        }
        ConfigSubcommand::Unset { key } => {
            match key {
                ConfigKey::ApiToken => file.api_token = None,
                ConfigKey::ApiUrl => file.api_url = None,
            }
            config::save(&path, &file)?;
            println!("Updated {}", path.display());
        }
    }

    Ok(())
}

fn subscribers(
    client: &ApiClient,
    format: OutputFormat,
    command: SubscribersCommand,
) -> Result<()> {
    match command.command {
        SubscribersSubcommand::List(args) => {
            let mut query = page_query(args.page);
            query.push_opt("sort", args.sort.as_ref().map(value_enum_string));
            query.push_opt("filter", args.filter.as_ref().map(value_enum_string));
            print_response(
                client,
                format,
                Method::GET,
                "/subscribers",
                query,
                Body::Empty,
            )
        }
        SubscribersSubcommand::Get(args) => print_response(
            client,
            format,
            Method::GET,
            &format!("/subscribers/{}", segment(&args.id)),
            QueryParams::default(),
            Body::Empty,
        ),
        SubscribersSubcommand::Add(args) => {
            let body = json!({ "email_address": args.email_address });
            print_response(
                client,
                format,
                Method::POST,
                "/subscribers",
                QueryParams::default(),
                Body::Json(&body),
            )
        }
        SubscribersSubcommand::Delete(args) => print_response_empty_ok(
            client,
            format,
            Method::DELETE,
            &format!("/subscribers/{}", segment(&args.id)),
            QueryParams::default(),
        ),
        SubscribersSubcommand::DeleteAll(args) => {
            if !args.confirm {
                bail!("delete-all requires --confirm");
            }
            let mut query = QueryParams::default();
            query.push("confirm", "true");
            print_response(
                client,
                format,
                Method::DELETE,
                "/subscribers",
                query,
                Body::Empty,
            )
        }
        SubscribersSubcommand::Imports(imports) => match imports.command {
            ImportsSubcommand::Create(args) => {
                if let Some(file) = args.file {
                    let response =
                        client.multipart_post_file("/subscribers/imports", "file", file)?;
                    print_value(format, &response_json(response))
                } else if !args.email_address.is_empty() {
                    let body = json!({ "email_addresses": args.email_address });
                    print_response(
                        client,
                        format,
                        Method::POST,
                        "/subscribers/imports",
                        QueryParams::default(),
                        Body::Json(&body),
                    )
                } else {
                    bail!("import create requires --file or at least one --email-address")
                }
            }
            ImportsSubcommand::Status(args) => print_response(
                client,
                format,
                Method::GET,
                &format!("/subscribers/imports/{}", segment(&args.id)),
                QueryParams::default(),
                Body::Empty,
            ),
        },
    }
}

fn account(client: &ApiClient, format: OutputFormat, command: AccountCommand) -> Result<()> {
    match command.command {
        AccountSubcommand::Get => print_response(
            client,
            format,
            Method::GET,
            "/account",
            QueryParams::default(),
            Body::Empty,
        ),
        AccountSubcommand::Update(args) => patch_json(client, format, "/account", "account", args),
        AccountSubcommand::Picture(command) => match command.command {
            PictureSubcommand::Get => print_response(
                client,
                format,
                Method::GET,
                "/account/picture",
                QueryParams::default(),
                Body::Empty,
            ),
            PictureSubcommand::Upload(args) => {
                let response =
                    client.multipart_file("/account/picture", "picture", args.picture)?;
                print_value(format, &response_json(response))
            }
            PictureSubcommand::Delete => print_response_empty_ok(
                client,
                format,
                Method::DELETE,
                "/account/picture",
                QueryParams::default(),
            ),
        },
        AccountSubcommand::Settings(command) => simple_json_resource(
            client,
            format,
            "/account/settings",
            "settings",
            command.command,
        ),
        AccountSubcommand::NewsletterSettings(command) => simple_json_resource(
            client,
            format,
            "/account/newsletter-settings",
            "newsletter_settings",
            command.command,
        ),
        AccountSubcommand::Themes(command) => themes(client, format, command),
        AccountSubcommand::Pages(command) => pages(client, format, command),
        AccountSubcommand::ConfirmationEmail(command) => simple_json_resource(
            client,
            format,
            "/account/confirmation-email",
            "confirmation_email",
            command.command,
        ),
    }
}

fn themes(client: &ApiClient, format: OutputFormat, command: ThemesCommand) -> Result<()> {
    match command.command {
        ThemesSubcommand::List(args) => {
            let mut query = page_query(args.page);
            query.push_opt("filter", args.filter.as_ref().map(value_enum_string));
            query.push_opt("sort", args.sort.as_ref().map(value_enum_string));
            print_response(
                client,
                format,
                Method::GET,
                "/account/themes",
                query,
                Body::Empty,
            )
        }
        ThemesSubcommand::Create(args) => {
            post_json(client, format, "/account/themes", "theme", args)
        }
        ThemesSubcommand::Get(args) => print_response(
            client,
            format,
            Method::GET,
            &format!("/account/themes/{}", segment(&args.id)),
            QueryParams::default(),
            Body::Empty,
        ),
        ThemesSubcommand::Update(args) => patch_json_by_id(
            client,
            format,
            "/account/themes",
            &args.id,
            "theme",
            args.body,
        ),
        ThemesSubcommand::Delete(args) => print_response_empty_ok(
            client,
            format,
            Method::DELETE,
            &format!("/account/themes/{}", segment(&args.id)),
            QueryParams::default(),
        ),
    }
}

fn pages(client: &ApiClient, format: OutputFormat, command: PagesCommand) -> Result<()> {
    let (page, command) = match command.command {
        PagesSubcommand::Archive(command) => ("archive-page", command),
        PagesSubcommand::Subscribe(command) => ("subscribe-page", command),
        PagesSubcommand::Confirmation(command) => ("confirmation-page", command),
        PagesSubcommand::Unsubscribe(command) => ("unsubscribe-page", command),
    };
    let root = format!("/account/{page}");
    let body_key = page.replace('-', "_");

    match command.command {
        PageSubcommand::Get => print_response(
            client,
            format,
            Method::GET,
            &root,
            QueryParams::default(),
            Body::Empty,
        ),
        PageSubcommand::Update(args) => patch_json(client, format, &root, &body_key, args),
        PageSubcommand::UploadBackground(args) => {
            let response = client.multipart_file(
                &format!("{root}/background-image"),
                "background_image",
                args.background_image,
            )?;
            print_value(format, &response_json(response))
        }
        PageSubcommand::DeleteBackground => print_response_empty_ok(
            client,
            format,
            Method::DELETE,
            &format!("{root}/background-image"),
            QueryParams::default(),
        ),
    }
}

fn newsletters(
    client: &ApiClient,
    format: OutputFormat,
    command: NewslettersCommand,
) -> Result<()> {
    match command.command {
        NewslettersSubcommand::List(args) => {
            let mut query = page_query(args.page);
            query.push("status", value_enum_string(&args.status));
            query.push_opt("sort", args.sort);
            print_response(
                client,
                format,
                Method::GET,
                "/newsletters",
                query,
                Body::Empty,
            )
        }
        NewslettersSubcommand::Create(args) => {
            let body = json_from_data(args.data, args.data_file)?;
            print_response(
                client,
                format,
                Method::POST,
                "/newsletters",
                QueryParams::default(),
                Body::Json(&body),
            )
        }
        NewslettersSubcommand::Get(args) => get_by_id(client, format, "/newsletters", &args.id),
        NewslettersSubcommand::Update(args) => {
            let body = json_from_data(args.body.data, args.body.data_file)?;
            print_response(
                client,
                format,
                Method::PATCH,
                &format!("/newsletters/{}", segment(&args.id)),
                QueryParams::default(),
                Body::Json(&body),
            )
        }
        NewslettersSubcommand::Delete(args) => {
            delete_by_id(client, format, "/newsletters", &args.id)
        }
        NewslettersSubcommand::Preview(args) => {
            action_by_id(client, format, "/newsletters", &args.id, "preview")
        }
        NewslettersSubcommand::Send(args) => {
            action_by_id(client, format, "/newsletters", &args.id, "send")
        }
        NewslettersSubcommand::Schedule(args) => {
            let body = json!({ "scheduled_for": args.scheduled_for });
            print_response(
                client,
                format,
                Method::POST,
                &format!("/newsletters/{}/schedule", segment(&args.id)),
                QueryParams::default(),
                Body::Json(&body),
            )
        }
        NewslettersSubcommand::CancelSchedule(args) => print_response(
            client,
            format,
            Method::DELETE,
            &format!("/newsletters/{}/schedule", segment(&args.id)),
            QueryParams::default(),
            Body::Empty,
        ),
        NewslettersSubcommand::Collaborators(command) => collaborators(client, format, command),
        NewslettersSubcommand::Lock(command) => locks(client, format, command),
        NewslettersSubcommand::Attachments(command) => match command.command {
            AttachmentsSubcommand::Upload(args) => {
                let response =
                    client.multipart_post_file("/newsletters/attachments", "file", args.file)?;
                print_value(format, &response_json(response))
            }
        },
    }
}

fn collaborators(
    client: &ApiClient,
    format: OutputFormat,
    command: CollaboratorsCommand,
) -> Result<()> {
    match command.command {
        CollaboratorsSubcommand::List(args) => print_response(
            client,
            format,
            Method::GET,
            &format!(
                "/newsletters/{}/collaborators",
                segment(&args.newsletter_id)
            ),
            QueryParams::default(),
            Body::Empty,
        ),
        CollaboratorsSubcommand::Invite(args) => {
            let body = json!({ "email_address": args.email_address });
            print_response(
                client,
                format,
                Method::POST,
                &format!(
                    "/newsletters/{}/collaborators",
                    segment(&args.newsletter_id)
                ),
                QueryParams::default(),
                Body::Json(&body),
            )
        }
        CollaboratorsSubcommand::Revoke(args) => print_response_empty_ok(
            client,
            format,
            Method::DELETE,
            &format!(
                "/newsletters/{}/collaborators/{}",
                segment(&args.newsletter_id),
                args.id
            ),
            QueryParams::default(),
        ),
    }
}

fn locks(client: &ApiClient, format: OutputFormat, command: LockCommand) -> Result<()> {
    match command.command {
        LockSubcommand::Acquire(args) => print_response(
            client,
            format,
            Method::POST,
            &format!("/newsletters/{}/lock", segment(&args.newsletter_id)),
            QueryParams::default(),
            Body::Empty,
        ),
        LockSubcommand::Release(args) => print_response_empty_ok(
            client,
            format,
            Method::DELETE,
            &format!("/newsletters/{}/lock", segment(&args.newsletter_id)),
            QueryParams::default(),
        ),
        LockSubcommand::Heartbeat(args) => print_response(
            client,
            format,
            Method::PATCH,
            &format!(
                "/newsletters/{}/lock/heartbeat",
                segment(&args.newsletter_id)
            ),
            QueryParams::default(),
            Body::Empty,
        ),
    }
}

fn shared_newsletters(
    client: &ApiClient,
    format: OutputFormat,
    command: SharedNewslettersCommand,
) -> Result<()> {
    match command.command {
        SharedNewslettersSubcommand::List(args) => {
            let mut query = page_query(args.page);
            query.push_opt("sort", args.sort);
            print_response(
                client,
                format,
                Method::GET,
                "/shared-newsletters",
                query,
                Body::Empty,
            )
        }
    }
}

fn automatic_newsletters(
    client: &ApiClient,
    format: OutputFormat,
    command: AutomaticNewslettersCommand,
) -> Result<()> {
    match command.command {
        AutomaticNewslettersSubcommand::List(args) => print_response(
            client,
            format,
            Method::GET,
            "/automatic-newsletters",
            page_query(args),
            Body::Empty,
        ),
        AutomaticNewslettersSubcommand::Create(args) => post_json(
            client,
            format,
            "/automatic-newsletters",
            "automatic_newsletter",
            args,
        ),
        AutomaticNewslettersSubcommand::Get(args) => {
            get_by_id(client, format, "/automatic-newsletters", &args.id)
        }
        AutomaticNewslettersSubcommand::Update(args) => patch_json_by_id(
            client,
            format,
            "/automatic-newsletters",
            &args.id,
            "automatic_newsletter",
            args.body,
        ),
        AutomaticNewslettersSubcommand::Delete(args) => {
            delete_by_id(client, format, "/automatic-newsletters", &args.id)
        }
        AutomaticNewslettersSubcommand::Pause(args) => {
            action_by_id(client, format, "/automatic-newsletters", &args.id, "pause")
        }
        AutomaticNewslettersSubcommand::Resume(args) => {
            action_by_id(client, format, "/automatic-newsletters", &args.id, "resume")
        }
        AutomaticNewslettersSubcommand::Validate(args) => {
            let body = json!({ "feed_url": args.feed_url });
            print_response(
                client,
                format,
                Method::POST,
                "/automatic-newsletters/validate",
                QueryParams::default(),
                Body::Json(&body),
            )
        }
    }
}

fn simple_json_resource(
    client: &ApiClient,
    format: OutputFormat,
    path: &str,
    key: &str,
    command: SimpleJsonSubcommand,
) -> Result<()> {
    match command {
        SimpleJsonSubcommand::Get => print_response(
            client,
            format,
            Method::GET,
            path,
            QueryParams::default(),
            Body::Empty,
        ),
        SimpleJsonSubcommand::Update(args) => patch_json(client, format, path, key, args),
    }
}

fn get_by_id(client: &ApiClient, format: OutputFormat, root: &str, id: &str) -> Result<()> {
    print_response(
        client,
        format,
        Method::GET,
        &format!("{root}/{}", segment(id)),
        QueryParams::default(),
        Body::Empty,
    )
}

fn delete_by_id(client: &ApiClient, format: OutputFormat, root: &str, id: &str) -> Result<()> {
    print_response_empty_ok(
        client,
        format,
        Method::DELETE,
        &format!("{root}/{}", segment(id)),
        QueryParams::default(),
    )
}

fn action_by_id(
    client: &ApiClient,
    format: OutputFormat,
    root: &str,
    id: &str,
    action: &str,
) -> Result<()> {
    print_response(
        client,
        format,
        Method::POST,
        &format!("{root}/{}/{}", segment(id), action),
        QueryParams::default(),
        Body::Empty,
    )
}

fn post_json(
    client: &ApiClient,
    format: OutputFormat,
    path: &str,
    key: &str,
    args: JsonBodyArgs,
) -> Result<()> {
    let body = wrap_body(key, json_from_data(args.data, args.data_file)?);
    print_response(
        client,
        format,
        Method::POST,
        path,
        QueryParams::default(),
        Body::Json(&body),
    )
}

fn patch_json(
    client: &ApiClient,
    format: OutputFormat,
    path: &str,
    key: &str,
    args: JsonBodyArgs,
) -> Result<()> {
    let body = wrap_body(key, json_from_data(args.data, args.data_file)?);
    print_response(
        client,
        format,
        Method::PATCH,
        path,
        QueryParams::default(),
        Body::Json(&body),
    )
}

fn patch_json_by_id(
    client: &ApiClient,
    format: OutputFormat,
    root: &str,
    id: &str,
    key: &str,
    args: JsonBodyArgs,
) -> Result<()> {
    let body = wrap_body(key, json_from_data(args.data, args.data_file)?);
    print_response(
        client,
        format,
        Method::PATCH,
        &format!("{root}/{}", segment(id)),
        QueryParams::default(),
        Body::Json(&body),
    )
}

fn print_response(
    client: &ApiClient,
    format: OutputFormat,
    method: Method,
    path: &str,
    query: QueryParams,
    body: Body<'_>,
) -> Result<()> {
    let response = client.request(method, path, &query, body)?;
    print_value(format, &response_json(response))
}

fn print_response_empty_ok(
    client: &ApiClient,
    format: OutputFormat,
    method: Method,
    path: &str,
    query: QueryParams,
) -> Result<()> {
    let response = client.request(method, path, &query, Body::Empty)?;
    print_value(format, &ensure_no_body(response))
}

fn page_query(args: PageArgs) -> QueryParams {
    let mut query = QueryParams::default();
    query.push_opt("page", args.page);
    query.push_opt("per_page", args.per_page);
    query
}

fn wrap_body(key: &str, value: Value) -> Value {
    if value.get(key).is_some() {
        value
    } else {
        json!({ key: value })
    }
}

fn segment(raw: &str) -> String {
    percent_encoding::utf8_percent_encode(raw, percent_encoding::NON_ALPHANUMERIC).to_string()
}

#[allow(dead_code)]
fn _assert_config_roundtrip(_: FileConfig) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_unwrapped_json_body() {
        assert_eq!(
            wrap_body("account", json!({ "name": "Ada" })),
            json!({ "account": { "name": "Ada" } })
        );
    }

    #[test]
    fn leaves_wrapped_json_body_alone() {
        assert_eq!(
            wrap_body("account", json!({ "account": { "name": "Ada" } })),
            json!({ "account": { "name": "Ada" } })
        );
    }

    #[test]
    fn encodes_path_segments() {
        assert_eq!(segment("tour announcement"), "tour%20announcement");
    }
}
