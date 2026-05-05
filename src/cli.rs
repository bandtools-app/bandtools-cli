use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use crate::config::OutputPreference;

#[derive(Debug, Parser)]
#[command(name = "bt", version, about = "BandTools command line interface")]
#[command(
    subcommand_required = true,
    arg_required_else_help = true,
    propagate_version = true
)]
pub struct Cli {
    #[arg(long, global = true, help = "BandTools API token")]
    pub api_token: Option<String>,

    #[arg(
        long,
        global = true,
        hide = true,
        help = "Override the BandTools API base URL"
    )]
    pub api_url: Option<String>,

    #[arg(long, global = true, help = "Path to the BandTools config file")]
    pub config: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        conflicts_with = "compact_json",
        help = "Emit pretty-printed JSON"
    )]
    pub json: bool,

    #[arg(long, global = true, help = "Emit compact JSON")]
    pub compact_json: bool,

    #[arg(
        long,
        global = true,
        conflicts_with_all = ["json", "compact_json"],
        help = "Emit plain text without TUI ornamentation"
    )]
    pub plain: bool,

    #[arg(long, global = true, help = "Disable colour in terminal output")]
    pub no_colour: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Manage subscribers")]
    Subscribers(SubscribersCommand),
    #[command(about = "Manage the current account")]
    Account(AccountCommand),
    #[command(about = "Manage newsletters")]
    Newsletters(NewslettersCommand),
    #[command(
        name = "shared-newsletters",
        about = "List newsletters shared with you"
    )]
    SharedNewsletters(SharedNewslettersCommand),
    #[command(name = "automatic-newsletters", about = "Manage automatic newsletters")]
    AutomaticNewsletters(AutomaticNewslettersCommand),
    #[command(about = "Manage local bt configuration")]
    Config(ConfigCommand),
    #[command(about = "Generate shell completion scripts")]
    Completions(CompletionsCommand),
}

#[derive(Debug, Args)]
pub struct CompletionsCommand {
    #[arg(value_enum, help = "Shell to generate completions for")]
    pub shell: Shell,
}

#[derive(Debug, Args)]
pub struct PageArgs {
    #[arg(long, help = "1-based page number")]
    pub page: Option<u32>,
    #[arg(long, help = "Items per page")]
    pub per_page: Option<u32>,
}

#[derive(Debug, Args)]
pub struct JsonBodyArgs {
    #[arg(long, help = "JSON request body")]
    pub data: Option<String>,
    #[arg(long, value_name = "PATH", help = "Read JSON request body from a file")]
    pub data_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct FileArg {
    #[arg(long, value_name = "PATH", help = "File to upload")]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct IdArg {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct NewsletterIdArg {
    #[arg(value_name = "NEWSLETTER_ID")]
    pub newsletter_id: String,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum SubscriberSort {
    EmailAsc,
    EmailDesc,
    SubscribedRecent,
    SubscribedOldest,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum SubscriberFilter {
    All,
    Confirmed,
    Unconfirmed,
}

#[derive(Debug, Args)]
pub struct SubscribersCommand {
    #[command(subcommand)]
    pub command: SubscribersSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SubscribersSubcommand {
    #[command(about = "List subscribers")]
    List(SubscribersListArgs),
    #[command(about = "Get a subscriber")]
    Get(IdArg),
    #[command(about = "Add a subscriber")]
    Add(SubscriberAddArgs),
    #[command(about = "Delete a subscriber")]
    Delete(IdArg),
    #[command(name = "delete-all", about = "Delete all subscribers")]
    DeleteAll(DeleteAllSubscribersArgs),
    #[command(about = "Manage subscriber imports")]
    Imports(ImportsCommand),
}

#[derive(Debug, Args)]
pub struct SubscribersListArgs {
    #[command(flatten)]
    pub page: PageArgs,
    #[arg(long, value_enum, help = "Sort order")]
    pub sort: Option<SubscriberSort>,
    #[arg(long, value_enum, help = "Confirmation status filter")]
    pub filter: Option<SubscriberFilter>,
}

#[derive(Debug, Args)]
pub struct SubscriberAddArgs {
    #[arg(long, help = "Email address to add")]
    pub email_address: String,
}

#[derive(Debug, Args)]
pub struct DeleteAllSubscribersArgs {
    #[arg(long, help = "Required confirmation for deleting all subscribers")]
    pub confirm: bool,
}

#[derive(Debug, Args)]
pub struct ImportsCommand {
    #[command(subcommand)]
    pub command: ImportsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ImportsSubcommand {
    #[command(about = "Import subscribers from CSV or JSON")]
    Create(SubscriberImportArgs),
    #[command(about = "Get subscriber import status")]
    Status(IdArg),
}

#[derive(Debug, Args)]
pub struct SubscriberImportArgs {
    #[arg(long, value_name = "PATH", conflicts_with = "email_address")]
    pub file: Option<PathBuf>,
    #[arg(long = "email-address", value_name = "EMAIL")]
    pub email_address: Vec<String>,
}

#[derive(Debug, Args)]
pub struct AccountCommand {
    #[command(subcommand)]
    pub command: AccountSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    #[command(about = "Get the current account")]
    Get,
    #[command(about = "Update the current account")]
    Update(JsonBodyArgs),
    #[command(about = "Manage the account picture")]
    Picture(PictureCommand),
    #[command(about = "Manage app settings")]
    Settings(SettingsCommand),
    #[command(name = "newsletter-settings", about = "Manage newsletter settings")]
    NewsletterSettings(NewsletterSettingsCommand),
    #[command(about = "Manage page themes")]
    Themes(ThemesCommand),
    #[command(about = "Manage page designs")]
    Pages(PagesCommand),
    #[command(name = "confirmation-email", about = "Manage the confirmation email")]
    ConfirmationEmail(ConfirmationEmailCommand),
}

#[derive(Debug, Args)]
pub struct PictureCommand {
    #[command(subcommand)]
    pub command: PictureSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PictureSubcommand {
    Get,
    Upload(PictureUploadArgs),
    Delete,
}

#[derive(Debug, Args)]
pub struct PictureUploadArgs {
    #[arg(long, value_name = "PATH")]
    pub picture: PathBuf,
}

#[derive(Debug, Args)]
pub struct SettingsCommand {
    #[command(subcommand)]
    pub command: SimpleJsonSubcommand,
}

#[derive(Debug, Args)]
pub struct NewsletterSettingsCommand {
    #[command(subcommand)]
    pub command: SimpleJsonSubcommand,
}

#[derive(Debug, Args)]
pub struct ConfirmationEmailCommand {
    #[command(subcommand)]
    pub command: SimpleJsonSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SimpleJsonSubcommand {
    Get,
    Update(JsonBodyArgs),
}

#[derive(Debug, Args)]
pub struct ThemesCommand {
    #[command(subcommand)]
    pub command: ThemesSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ThemesSubcommand {
    List(ThemesListArgs),
    Create(JsonBodyArgs),
    Get(IdArg),
    Update(UpdateByIdArgs),
    Delete(IdArg),
}

#[derive(Debug, Args)]
pub struct ThemesListArgs {
    #[command(flatten)]
    pub page: PageArgs,
    #[arg(long, value_enum)]
    pub filter: Option<ThemeFilter>,
    #[arg(long, value_enum)]
    pub sort: Option<ThemeSort>,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum ThemeFilter {
    All,
    User,
    System,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum ThemeSort {
    NameAsc,
    NameDesc,
    CreatedAsc,
    CreatedDesc,
}

#[derive(Debug, Args)]
pub struct UpdateByIdArgs {
    pub id: String,
    #[command(flatten)]
    pub body: JsonBodyArgs,
}

#[derive(Debug, Args)]
pub struct PagesCommand {
    #[command(subcommand)]
    pub command: PagesSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PagesSubcommand {
    Archive(PageCommand),
    Subscribe(PageCommand),
    Confirmation(PageCommand),
    Unsubscribe(PageCommand),
}

#[derive(Debug, Args)]
pub struct PageCommand {
    #[command(subcommand)]
    pub command: PageSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PageSubcommand {
    Get,
    Update(JsonBodyArgs),
    #[command(name = "upload-background")]
    UploadBackground(BackgroundUploadArgs),
    #[command(name = "delete-background")]
    DeleteBackground,
}

#[derive(Debug, Args)]
pub struct BackgroundUploadArgs {
    #[arg(long, value_name = "PATH")]
    pub background_image: PathBuf,
}

#[derive(Debug, Args)]
pub struct NewslettersCommand {
    #[command(subcommand)]
    pub command: NewslettersSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum NewslettersSubcommand {
    List(NewslettersListArgs),
    Create(JsonBodyArgs),
    Get(IdArg),
    Update(UpdateByIdArgs),
    Delete(IdArg),
    Preview(IdArg),
    Send(IdArg),
    Schedule(ScheduleArgs),
    #[command(name = "cancel-schedule")]
    CancelSchedule(IdArg),
    Collaborators(CollaboratorsCommand),
    Lock(LockCommand),
    Attachments(AttachmentsCommand),
}

#[derive(Debug, Args)]
pub struct NewslettersListArgs {
    #[arg(long, value_enum)]
    pub status: NewsletterStatus,
    #[command(flatten)]
    pub page: PageArgs,
    #[arg(long)]
    pub sort: Option<String>,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum NewsletterStatus {
    Draft,
    Sent,
    Scheduled,
}

#[derive(Debug, Args)]
pub struct ScheduleArgs {
    pub id: String,
    #[arg(long)]
    pub scheduled_for: String,
}

#[derive(Debug, Args)]
pub struct CollaboratorsCommand {
    #[command(subcommand)]
    pub command: CollaboratorsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CollaboratorsSubcommand {
    List(NewsletterIdArg),
    Invite(CollaboratorInviteArgs),
    Revoke(CollaboratorRevokeArgs),
}

#[derive(Debug, Args)]
pub struct CollaboratorInviteArgs {
    #[arg(value_name = "NEWSLETTER_ID")]
    pub newsletter_id: String,
    #[arg(long)]
    pub email_address: String,
}

#[derive(Debug, Args)]
pub struct CollaboratorRevokeArgs {
    #[arg(value_name = "NEWSLETTER_ID")]
    pub newsletter_id: String,
    pub id: u64,
}

#[derive(Debug, Args)]
pub struct LockCommand {
    #[command(subcommand)]
    pub command: LockSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum LockSubcommand {
    Acquire(NewsletterIdArg),
    Release(NewsletterIdArg),
    Heartbeat(NewsletterIdArg),
}

#[derive(Debug, Args)]
pub struct AttachmentsCommand {
    #[command(subcommand)]
    pub command: AttachmentsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AttachmentsSubcommand {
    Upload(FileArg),
}

#[derive(Debug, Args)]
pub struct SharedNewslettersCommand {
    #[command(subcommand)]
    pub command: SharedNewslettersSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SharedNewslettersSubcommand {
    List(SharedNewslettersListArgs),
}

#[derive(Debug, Args)]
pub struct SharedNewslettersListArgs {
    #[command(flatten)]
    pub page: PageArgs,
    #[arg(long)]
    pub sort: Option<String>,
}

#[derive(Debug, Args)]
pub struct AutomaticNewslettersCommand {
    #[command(subcommand)]
    pub command: AutomaticNewslettersSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AutomaticNewslettersSubcommand {
    List(PageArgs),
    Create(JsonBodyArgs),
    Get(IdArg),
    Update(UpdateByIdArgs),
    Delete(IdArg),
    Pause(IdArg),
    Resume(IdArg),
    Validate(FeedValidateArgs),
}

#[derive(Debug, Args)]
pub struct FeedValidateArgs {
    #[arg(long)]
    pub feed_url: String,
}

#[derive(Debug, Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    #[command(about = "Show the resolved config path")]
    Path,
    #[command(about = "Show local config values without revealing token")]
    Show,
    #[command(about = "Set api_token in the local config file")]
    SetToken { api_token: String },
    #[command(about = "Set api_url in the local config file")]
    SetApiUrl { api_url: String },
    #[command(about = "Set preferred response output in the local config file")]
    SetOutput { output: ConfigOutput },
    #[command(about = "Clear api_token, api_url, or output from the local config file")]
    Unset { key: ConfigKey },
}

#[derive(Debug, ValueEnum, Clone)]
pub enum ConfigKey {
    ApiToken,
    ApiUrl,
    Output,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
pub enum ConfigOutput {
    Tui,
    Plain,
    Json,
    CompactJson,
}

impl From<ConfigOutput> for OutputPreference {
    fn from(output: ConfigOutput) -> Self {
        match output {
            ConfigOutput::Tui => Self::Tui,
            ConfigOutput::Plain => Self::Plain,
            ConfigOutput::Json => Self::Json,
            ConfigOutput::CompactJson => Self::CompactJson,
        }
    }
}

pub fn value_enum_string<T: ValueEnum + Clone>(value: &T) -> String {
    value
        .to_possible_value()
        .unwrap()
        .get_name()
        .replace('-', "_")
}
