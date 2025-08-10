use poise::serenity_prelude::*;
use tracing::info;
pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        // for guild in ready.guilds {
        //     info!(
        //         "Ready to verify users in guild: {}",
        //         guild
        //             .id
        //             .name(&ctx.cache)
        //             .get_or_insert("Unknown guild".into())
        //     );
        // }
    }
    // async fn guild_member_addition(&self, ctx: serenity::Context, new_member: serenity::Member) {
    //     info!("New member: {}", new_member.user.id);
    //     if db::is_verified(&new_member.guild_id).await {
    //         info!("Already verified {}, welcome back!", new_member.user.id);
    //         add_verify_roles(&ctx, &new_member)
    //             .await
    //             .unwrap_or_else(|err| error!("Error adding verified role: {}", err));
    //         STAFF_CHANNEL
    //             .say(&ctx, "Already verified. Welcome back!")
    //             .await
    //             .map(|_| ())
    //             .unwrap_or_else(|err| error!("Error sending message: {}", err));
    //     }
    //     queue_verification(&ctx, &new_member)
    //         .await
    //         .unwrap_or_else(|_| error!("Could not notify user about verification"));
    // }
}
