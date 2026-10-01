use pumpkin_plugin_api::{
    command::{Command, CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    Context, Plugin, PluginMetadata, Server,
};

/// Forwards an Essentials-style alias to a normal Pumpkin command.
/// `{player}` is replaced with the command sender's name.
struct Forward(&'static str);

impl CommandHandler for Forward {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player_name = sender.get_name();
        let command = self.0.replace("{player}", &player_name);

        server.execute_command(
            &command,
            pumpkin_plugin_api::server::CommandSender::Console,
        );

        Ok(1)
    }
}

fn register_forward(context: &Context, names: &[&str], command: &'static str) {
    let aliases = names.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();

    let node = Command::new(
        aliases,
        "Essentials-style command",
    )
    .execute(Forward(command));

    context.register_command(node, "command");
}

struct EssentialsPumpkin;

impl Plugin for EssentialsPumpkin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "essentials_pumpkin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Hekodeko".into()],
            description: "Essentials-style commands for PumpkinMC.".into(),
            dependencies: vec![],
            permissions: vec![],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        // Gamemode
        register_forward(&context, &["gmc", "creative"], "gamemode creative {player}");
        register_forward(&context, &["gms", "survival"], "gamemode survival {player}");
        register_forward(&context, &["gma", "adventure"], "gamemode adventure {player}");
        register_forward(&context, &["gmsp", "spectator"], "gamemode spectator {player}");

        // Time / weather
        register_forward(&context, &["day"], "time set day");
        register_forward(&context, &["night"], "time set night");
        register_forward(&context, &["sun"], "weather clear");
        register_forward(&context, &["rain"], "weather rain");
        register_forward(&context, &["thunder"], "weather thunder");

        // Player utility aliases
        register_forward(&context, &["kill", "suicide"], "kill {player}");
        register_forward(&context, &["clear"], "clear {player}");
        register_forward(&context, &["fly"], "fly {player}");

        // Common Essentials aliases for commands already provided by the server.
        register_forward(&context, &["tp"], "tp {player}");
        register_forward(&context, &["spawn"], "spawn {player}");
        register_forward(&context, &["help"], "help");
        register_forward(&context, &["plugins"], "plugins");
        register_forward(&context, &["rules"], "rules");

        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(EssentialsPumpkin);
