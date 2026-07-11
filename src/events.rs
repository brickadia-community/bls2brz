//! Blockland `+-EVENT` records translated into Brickadia brick components.
//!
//! For now only the `CenterPrint` output is supported: a Blockland brick that
//! center-prints a message when activated becomes a Brickadia
//! `Component_Interact`, whose `Message` shows when a player interacts with
//! (clicks) the brick.
//!
//! Blockland and Brickadia both mark up colored text inline, but with different
//! syntax, so the message string is rewritten in [`to_brickadia_markup`]:
//!   - `<color:RRGGBB>` opening tags become `<color="RRGGBB">`
//!   - `</color>` closing tags become `</>`
//!   - the whole message is wrapped in a default-red tag, since Blockland
//!     center-prints render untagged text red.

use bls::Event;
use brdb::assets::LiteralComponent;
use brdb::AsBrdbValue;
use lazy_static::lazy_static;
use regex::Regex;

/// Blockland center-prints render untagged text in red by default; wrapping the
/// whole message reproduces that (and matches the game's own `interact.brz`).
const DEFAULT_COLOR: &str = "ff0000";

lazy_static! {
    /// Blockland's `<color:RRGGBB>` opening tag.
    static ref OPEN_TAG: Regex = Regex::new(r"<color:([0-9a-fA-F]{6})>").unwrap();
}

/// If `event` is a supported `onActivate` → `CenterPrint` event, return the
/// Brickadia-markup message it should show; otherwise `None`. The `onActivate`
/// input maps to a player clicking the brick (a `Component_Interact`); other
/// inputs (touch, wrench, ...) have no equivalent here and are skipped.
pub fn centerprint_message(event: &Event) -> Option<String> {
    if !event.input.eq_ignore_ascii_case("onActivate")
        || !event.output.eq_ignore_ascii_case("CenterPrint")
    {
        return None;
    }
    let text = event.params.first().map(String::as_str).unwrap_or("");
    Some(to_brickadia_markup(text))
}

/// Rewrite Blockland inline color markup into Brickadia's, wrapping the result
/// in the default red so untagged text keeps Blockland's default color.
fn to_brickadia_markup(text: &str) -> String {
    let converted = OPEN_TAG
        .replace_all(text, "<color=\"$1\">")
        .replace("</color>", "</>");
    format!("<color=\"{DEFAULT_COLOR}\">{converted}</>")
}

/// Build the `Component_Interact` component showing `message` on interaction.
/// Direct interaction only (no nearby-interaction), mirroring Blockland's
/// click-to-print behavior; the writer fills the rest from defaults.
pub fn interact_component(message: &str) -> LiteralComponent {
    LiteralComponent::new("Component_Interact").with_data([
        ("Message", Box::new(message.to_string()) as Box<dyn AsBrdbValue>),
        ("bAllowNearbyInteraction", Box::new(false)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(output: &str, params: &[&str]) -> Event {
        ev_in("onActivate", output, params)
    }

    fn ev_in(input: &str, output: &str, params: &[&str]) -> Event {
        let mut e = Event::new(input, output);
        e.params = params.iter().map(|s| s.to_string()).collect();
        e
    }

    #[test]
    fn plain_message_wrapped_red() {
        let msg = centerprint_message(&ev("CenterPrint", &["test", "3"])).unwrap();
        assert_eq!(msg, r#"<color="ff0000">test</>"#);
    }

    #[test]
    fn color_tags_converted() {
        let msg = centerprint_message(&ev(
            "CenterPrint",
            &["<color:00ff00>test</color><color:0000ff>colors</color>", "3"],
        ))
        .unwrap();
        assert_eq!(
            msg,
            r#"<color="ff0000"><color="00ff00">test</><color="0000ff">colors</></>"#
        );
    }

    #[test]
    fn non_centerprint_skipped() {
        assert!(centerprint_message(&ev("addItem", &["gun"])).is_none());
    }

    #[test]
    fn non_activate_input_skipped() {
        assert!(centerprint_message(&ev_in("onPlayerTouch", "CenterPrint", &["hi"])).is_none());
    }
}
