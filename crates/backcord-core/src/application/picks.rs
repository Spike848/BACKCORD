//! Several people picked at once with Ctrl, whom one menu acts on
//! together: in a voice channel's list or a server's member list.

use super::*;

/// Someone picked, with the place their menu acts in: the voice channel
/// that lists them, or the channel whose member list shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picked {
    pub place: String,
    pub user: String,
}

/// What a menu of several people offers, of what each one's own offers:
/// what acts on each the same, and none of what's about one person (their
/// profile, a message or a call, a note, a nickname, their roles, an ID).
const TOGETHER: [MenuChoice; 18] = [
    MenuChoice::Mention,
    MenuChoice::AddFriend,
    MenuChoice::Unfriend,
    MenuChoice::Ignore,
    MenuChoice::Unignore,
    MenuChoice::Block,
    MenuChoice::Unblock,
    MenuChoice::LocalMute,
    MenuChoice::SoundboardMute,
    MenuChoice::BypassEffects,
    MenuChoice::MoveTo,
    MenuChoice::ServerMute,
    MenuChoice::ServerDeafen,
    MenuChoice::Disconnect,
    MenuChoice::Timeout,
    MenuChoice::RemoveTimeout,
    MenuChoice::Kick,
    MenuChoice::Ban,
];

/// What a menu of several people offers when only some of them have it in
/// their own menus, acting on those: a friend request to those who aren't
/// friends yet, and unfriending those who are.
const ANY_OF: [MenuChoice; 2] = [MenuChoice::AddFriend, MenuChoice::Unfriend];

impl Application {
    /// Picks someone along with the rest, or lets them go if picked; one
    /// from another server than those picked starts the picking anew.
    pub(super) fn pick(&mut self, place: String, user: String) -> Vec<Effect> {
        let server = self.pick_server(&place);
        let first = self.state.picked.first().map(|first| first.place.clone());
        if first.is_some_and(|first| first != place && self.pick_server(&first) != server) {
            self.state.picked.clear();
        }
        let picked = &mut self.state.picked;
        match picked.iter().position(|pick| pick.user == user) {
            Some(at) => {
                picked.remove(at);
            }
            None => picked.push(Picked { place, user }),
        }
        vec![]
    }

    /// The Discord server a place is in, by its id.
    fn pick_server(&self, place: &str) -> Option<String> {
        #[cfg(feature = "discord")]
        return self
            .discord_place(place)
            .map(|(server, _, _)| server.id.clone());
        #[cfg(not(feature = "discord"))]
        {
            let _ = place;
            None
        }
    }

    /// What everyone picked has in their own menus that acts on them all,
    /// in the first one's order; nothing for fewer than two.
    pub fn picked_choices(&self) -> Vec<MenuChoice> {
        let picked = &self.state.picked;
        if picked.len() < 2 {
            return vec![];
        }
        let each: Vec<Vec<MenuChoice>> = picked.iter().map(|pick| self.choices_of(pick)).collect();
        let mut choices: Vec<MenuChoice> = each[0]
            .iter()
            .copied()
            .filter(|choice| TOGETHER.contains(choice))
            .filter(|choice| {
                ANY_OF.contains(choice) || each[1..].iter().all(|other| other.contains(choice))
            })
            .filter(|choice| {
                *choice != MenuChoice::MoveTo || !self.picked_move_targets().is_empty()
            })
            .collect();
        // A friend request, or an unfriending, for those it suits, though
        // the first picked isn't among them: after Mention, as in each
        // one's own menu.
        for choice in ANY_OF {
            if !choices.contains(&choice) && each.iter().any(|own| own.contains(&choice)) {
                let at = choices
                    .iter()
                    .position(|choice| *choice == MenuChoice::Mention)
                    .map_or(0, |at| at + 1);
                choices.insert(at, choice);
            }
        }
        choices
    }

    /// Those picked whose own menus offer `choice`: whom a friend request
    /// or an unfriending of them all goes to.
    pub fn picked_with(&self, choice: MenuChoice) -> Vec<Picked> {
        self.state
            .picked
            .iter()
            .filter(|pick| self.choices_of(pick).contains(&choice))
            .cloned()
            .collect()
    }

    /// Someone's own menu where they were picked.
    fn choices_of(&self, pick: &Picked) -> Vec<MenuChoice> {
        #[cfg(feature = "discord")]
        if self.voice_person(&pick.place, &pick.user).is_some() {
            return self.voice_person_choices(&pick.place, &pick.user);
        }
        #[cfg(feature = "discord")]
        if self.is_member_place(&pick.place, &pick.user) {
            return self.member_choices(&pick.place, &pick.user);
        }
        self.state.person_choices(&pick.user)
    }

    /// The voice channels everyone picked may be moved to, by room id and
    /// name.
    pub fn picked_move_targets(&self) -> Vec<(String, String)> {
        let mut picked = self.state.picked.iter();
        let Some(first) = picked.next() else {
            return vec![];
        };
        let mut targets = self.move_targets(&first.place, &first.user);
        for pick in picked {
            let theirs = self.move_targets(&pick.place, &pick.user);
            targets.retain(|target| theirs.contains(target));
        }
        targets
    }

    /// Moves everyone picked to the voice channel `to`, each as their own
    /// menu's Move to would.
    pub fn move_picked(&self, to: &str) -> Vec<Action> {
        self.state
            .picked
            .iter()
            .filter_map(|pick| self.move_to(&pick.place, &pick.user, to))
            .collect()
    }

    /// The name a dialog gives someone picked.
    fn picked_name(&self, pick: &Picked) -> String {
        self.voice_person(&pick.place, &pick.user)
            .map(|person| person.name.clone())
            .or_else(|| self.state.person(&pick.user).map(|person| person.name))
            .unwrap_or_else(|| pick.user.clone())
    }

    /// Opens the dialog asking `ask` of everyone picked whom this account
    /// may ask it of: a timeout, a kick or a ban, as each one's own menu
    /// allows it, or an unfriending or a block. Its answer acts on them all.
    pub(super) fn ask_picked(&mut self, ask: PersonAsk) -> Vec<Effect> {
        let picked = self.state.picked.clone();
        let asked: Vec<Picked> = match ask {
            #[cfg(feature = "discord")]
            PersonAsk::Timeout | PersonAsk::Kick | PersonAsk::Ban => picked
                .into_iter()
                .filter(|pick| self.member_ask_allowed(&pick.place, &pick.user, ask))
                .collect(),
            PersonAsk::Unfriend => self.picked_with(MenuChoice::Unfriend),
            PersonAsk::Block => picked
                .into_iter()
                .filter(|pick| self.state.relation(&pick.user) != Relation::Me)
                .collect(),
            _ => return vec![],
        };
        let Some((first, rest)) = asked.split_first() else {
            return vec![];
        };
        let effects = match ask {
            PersonAsk::Unfriend | PersonAsk::Block => {
                self.ask_person(first.user.clone(), ask, "ContextMenu")
            }
            _ => self.ask_member(first.place.clone(), first.user.clone(), ask),
        };
        let names: Vec<String> = asked.iter().map(|pick| self.picked_name(pick)).collect();
        if let Some(dialog) = self
            .state
            .person_dialog
            .as_mut()
            .filter(|_| !rest.is_empty())
        {
            dialog.others = rest.iter().map(|pick| pick.user.clone()).collect();
            dialog.name = together(&names);
        }
        effects
    }
}

/// Several names in a dialog's words: up to three, then how many more.
fn together(names: &[String]) -> String {
    match names {
        [first, second, third, rest @ ..] if !rest.is_empty() => {
            format!("{first}, {second}, {third} +{}", rest.len())
        }
        _ => names.join(", "),
    }
}

#[cfg(all(test, feature = "discord"))]
mod tests {
    use super::*;
    use crate::application::member_server as server;
    use crate::chat::DiscordCommand;

    const GENERAL: &str = "discord:10:11";
    const LOUNGE: &str = "discord:10:12";

    fn pick(app: &mut Application, place: &str, user: &str) {
        app.dispatch(Action::Pick {
            place: place.into(),
            user: user.into(),
        });
    }

    #[test]
    fn picking_toggles_and_another_server_starts_anew() {
        let mut app = server(&["30"]);
        pick(&mut app, GENERAL, "4");
        pick(&mut app, GENERAL, "7");
        assert_eq!(app.state.picked.len(), 2);
        pick(&mut app, GENERAL, "7");
        assert_eq!(app.state.picked.len(), 1, "a second click lets go");
        pick(&mut app, "!room:test", "@bob:test");
        assert_eq!(app.state.picked.len(), 1, "elsewhere starts anew");
        app.dispatch(Action::ClearPicks);
        assert!(app.state.picked.is_empty());
    }

    #[test]
    fn several_get_what_acts_on_each_and_nothing_about_one() {
        let mut app = server(&["30"]);
        // Dave and Gus: Gus is an administrator, so no timeout.
        pick(&mut app, GENERAL, "4");
        pick(&mut app, GENERAL, "6");
        let choices = app.picked_choices();
        assert!(
            choices.ends_with(&[MenuChoice::Kick, MenuChoice::Ban]),
            "{choices:?}"
        );
        assert!(!choices.contains(&MenuChoice::Timeout));
        assert!(!choices.contains(&MenuChoice::Profile));
        assert!(!choices.contains(&MenuChoice::Roles));
        assert!(!choices.contains(&MenuChoice::CopyId));
        // One alone has no menu of several.
        app.dispatch(Action::ClearPicks);
        pick(&mut app, GENERAL, "4");
        assert!(app.picked_choices().is_empty());
    }

    /// A friend request and an unfriending show when any of them suits,
    /// and go to those it suits alone.
    #[test]
    fn a_friend_request_of_several_goes_to_those_not_friends() {
        let mut app = server(&["30"]);
        for user in ["4", "6", "3"] {
            pick(&mut app, GENERAL, user);
        }
        let choices = app.picked_choices();
        for choice in ANY_OF {
            let suited: Vec<String> = app
                .state
                .picked
                .clone()
                .iter()
                .filter(|pick| app.choices_of(pick).contains(&choice))
                .map(|pick| pick.user.clone())
                .collect();
            assert_eq!(choices.contains(&choice), !suited.is_empty(), "{choice:?}");
            let sent: Vec<String> = app
                .picked_with(choice)
                .into_iter()
                .map(|pick| pick.user)
                .collect();
            assert_eq!(sent, suited);
        }
    }

    #[test]
    fn a_kick_of_several_sends_each_and_waits_for_every_answer() {
        let mut app = server(&["30"]);
        pick(&mut app, GENERAL, "4");
        pick(&mut app, GENERAL, "6");
        // Cara is above her: left out of the kick.
        pick(&mut app, GENERAL, "3");
        app.dispatch(Action::AskPicked(PersonAsk::Kick));
        let dialog = app.state.person_dialog.clone().expect("the dialog");
        assert_eq!(dialog.others, ["6"]);
        assert_eq!(dialog.name, "Dave, Gus");
        let effects = app.dispatch(Action::AnswerPerson("raid".into()));
        let kicked: Vec<&str> = effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Backend(Command::Discord(DiscordCommand::Member { user, .. })) => {
                    Some(user.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(kicked, ["4", "6"]);
        app.apply(Event::PersonSaved(Ok(())));
        assert!(
            app.state.person_dialog.is_some(),
            "one answer still to come"
        );
        app.apply(Event::PersonSaved(Ok(())));
        assert!(app.state.person_dialog.is_none());
        assert!(app.state.picked.is_empty(), "done, the picking ends");
    }

    #[test]
    fn several_in_a_call_move_together_where_all_may_go() {
        let mut app = server(&["30"]);
        pick(&mut app, LOUNGE, "4");
        pick(&mut app, LOUNGE, "1");
        assert_eq!(
            app.picked_move_targets()
                .into_iter()
                .map(|(_, name)| name)
                .collect::<Vec<_>>(),
            ["Den"]
        );
        assert_eq!(app.move_picked("discord:10:13").len(), 2);
        assert!(
            app.move_picked("discord:10:14").is_empty(),
            "Vault is closed"
        );
    }
}
