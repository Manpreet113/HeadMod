use crate::types::{Context, Error};
use poise::serenity_prelude as serenity;

/// Interactive Embed Builder for professional announcements.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MANAGE_MESSAGES",
    description_localized("en-US", "Open the interactive Head Mod Embed Builder.")
)]
pub async fn embed(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let interaction_id = ctx.id();
    
    let draft = serenity::CreateEmbed::new()
        .title("Untitled Embed")
        .description("Use the tools below to design your message.")
        .colour(serenity::Colour::BLURPLE);

    loop {
        let embed_clone = draft.clone();
        let components = vec![
            serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(format!("{}_title", interaction_id)).label("Set Title").style(serenity::ButtonStyle::Secondary),
                serenity::CreateButton::new(format!("{}_desc", interaction_id)).label("Set Description").style(serenity::ButtonStyle::Secondary),
                serenity::CreateButton::new(format!("{}_color", interaction_id)).label("Set Color").style(serenity::ButtonStyle::Secondary),
            ]),
            serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(format!("{}_field", interaction_id)).label("Add Field").style(serenity::ButtonStyle::Secondary),
                serenity::CreateButton::new(format!("{}_image", interaction_id)).label("Set Image").style(serenity::ButtonStyle::Secondary),
                serenity::CreateButton::new(format!("{}_footer", interaction_id)).label("Set Footer").style(serenity::ButtonStyle::Secondary),
            ]),
            serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(format!("{}_post", interaction_id)).label("Post Embed").style(serenity::ButtonStyle::Success).emoji('🚀'),
                serenity::CreateButton::new(format!("{}_cancel", interaction_id)).label("Discard").style(serenity::ButtonStyle::Danger),
            ]),
        ];

        ctx.send(poise::CreateReply::default().embed(embed_clone).components(components)).await?;

        let mci = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
            .author_id(ctx.author().id)
            .timeout(std::time::Duration::from_secs(600))
            .filter(move |i| i.data.custom_id.starts_with(&interaction_id.to_string()))
            .await;

        match mci {
            Some(mci) => {
                mci.defer(ctx.http()).await?;
                let custom_id = mci.data.custom_id.replace(&format!("{}_", interaction_id), "");
                match custom_id.as_str() {
                    "cancel" => {
                        let _ = mci.edit_response(&ctx.http(), serenity::EditInteractionResponse::new().content("❌ Embed creation discarded.").embeds(vec![]).components(vec![])).await;
                        return Ok(());
                    },
                    "post" => {
                        let _ = mci.edit_response(&ctx.http(), serenity::EditInteractionResponse::new().content("✅ Posting tool is being optimized.")).await;
                    },
                    _ => {
                        let _ = mci.edit_response(&ctx.http(), serenity::EditInteractionResponse::new().content("🔄 Input handling via Modals is coming in the next build.")).await;
                    }
                }
            },
            None => break,
        }
    }
    Ok(())
}
