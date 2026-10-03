//! Plain-language descriptions of effects and items for the player's screens.

use crate::content::{Condition, Content, Effect, Scope};

fn signed(v: f32) -> String {
    if v >= 0.0 { format!("+{v:.0}") } else { format!("{v:.0}") }
}

fn need(c: &Content, id: &str) -> String {
    c.needs.get(id).map_or(id.to_string(), |n| n.name.to_lowercase())
}

fn stat(c: &Content, id: &str) -> String {
    c.stats.get(id).map_or(id.to_string(), |s| s.name.to_lowercase())
}

fn status(c: &Content, id: &str) -> String {
    c.statuses.get(id).map_or(id.to_string(), |s| s.name.clone())
}

fn item(c: &Content, id: &str) -> String {
    c.items.get(id).map_or(id.to_string(), |s| s.name.clone())
}

/// One effect in a short sentence ("sazia la fame (+40%)", "rende Brillo"…).
pub fn effect(c: &Content, e: &Effect) -> String {
    match e {
        Effect::On(scope, inner) => {
            let who = match scope {
                Scope::Subject => "",
                Scope::Target => "sul bersaglio: ",
                Scope::SubjectFaction => "su tutta la fazione: ",
                Scope::TargetFaction => "sulla fazione del bersaglio: ",
                _ => "su chi è intorno: ",
            };
            format!("{who}{}", effect(c, inner))
        }
        Effect::All(list) => list.iter().map(|x| effect(c, x)).collect::<Vec<_>>().join(", "),
        Effect::Chance(p, inner) => format!("{:.0}% di probabilità: {}", p * 100.0, effect(c, inner)),
        Effect::If(_, inner) => format!("a volte: {}", effect(c, inner)),
        Effect::IfElse(_, a, b) => format!("{} oppure {}", effect(c, a), effect(c, b)),
        Effect::ModStat { stat: s, amount } => format!("{} {}", stat(c, s), signed(*amount)),
        Effect::SetStat { stat: s, value } => format!("{} a {value:.0}", stat(c, s)),
        Effect::ModNeed { need: n, amount } if *amount >= 0.0 => format!("soddisfa {} (+{:.0}%)", need(c, n), amount * 100.0),
        Effect::ModNeed { need: n, amount } => format!("peggiora {} ({:.0}%)", need(c, n), amount * 100.0),
        Effect::ApplyStatus { status: s, .. } => format!("rende {}", status(c, s)),
        Effect::RemoveStatus(s) => format!("cura {}", status(c, s)),
        Effect::RemoveStatusTag(t) => format!("cura i malanni di tipo {t}"),
        Effect::Immunize { status: s, ticks } if *ticks > 0 => format!("protegge da {} per {ticks} ore", status(c, s)),
        Effect::Immunize { status: s, .. } => format!("protegge per sempre da {}", status(c, s)),
        Effect::GiveItem { item: i, qty } => format!("dà {qty}× {}", item(c, i)),
        Effect::GiveRandomItem { tag: Some(t), .. } => format!("dà un pezzo a caso della serie «{t}»"),
        Effect::GiveRandomItem { .. } => "dà un oggetto a sorpresa".into(),
        Effect::TakeItem { item: i, qty } => format!("toglie {qty}× {}", item(c, i)),
        Effect::ModMoney(m) => format!("soldi {m:+.0} €"),
        Effect::ModTreasury(m) => format!("fondo di gilda {m:+.0} €"),
        Effect::ModRelation { faction, amount } => {
            format!("rapporti con {} {}", c.factions.get(faction).map_or(faction.as_str(), |f| f.name.as_str()), signed(*amount))
        }
        Effect::AddWanted { .. } => "ti rende più ricercato".into(),
        Effect::ClearWanted => "cancella le accuse".into(),
        Effect::Damage { amount, .. } => format!("ferisce ({amount:.0} danni)"),
        Effect::Heal(h) => format!("cura le ferite (+{h:.0})"),
        Effect::Kill => "uccide".into(),
        Effect::Transmute(r) => format!("trasforma in {}", c.races.get(r).map_or(r.as_str(), |x| x.name.as_str())),
        Effect::AddClass(k) => format!("insegna il mestiere di {}", c.classes.get(k).map_or(k.as_str(), |x| x.name.as_str())),
        Effect::RemoveClass(k) => format!("fa dimenticare il mestiere di {}", c.classes.get(k).map_or(k.as_str(), |x| x.name.as_str())),
        Effect::VictoryPoints(v) => format!("{v:+} punti vittoria"),
        Effect::ModDissent(d) => format!("dissenso {}", signed(*d)),
        Effect::ModCover(v) => format!("copertura {}", signed(*v)),
        Effect::Stealth { duration, .. } => format!("rende invisibile per {duration} ore"),
        Effect::Shapeshift { .. } => "cambia aspetto".into(),
        Effect::RevertForm => "riporta all'aspetto vero".into(),
        Effect::Expose => "smaschera".into(),
        Effect::Clean(_) => "pulisce intorno".into(),
        Effect::Teleport { .. } => "teletrasporta".into(),
        Effect::ModBond { friendship, .. } if *friendship != 0.0 => format!("amicizia {}", signed(*friendship)),
        Effect::ModBond { attraction, .. } => format!("attrazione {}", signed(*attraction)),
        Effect::TransmuteFor { race, ticks } => format!("trasforma in {} per {ticks} ore", c.races.get(race).map_or(race.as_str(), |x| x.name.as_str())),
        _ => "effetto speciale".into(),
    }
}

/// What an item does, line by line: when used, while carried, for victory.
pub fn item_effects(c: &Content, id: &str) -> Vec<String> {
    let Some(d) = c.items.get(id) else { return vec![] };
    let mut out = Vec::new();
    if !d.on_use.is_empty() {
        let what = d.on_use.iter().map(|e| effect(c, e)).collect::<Vec<_>>().join(", ");
        out.push(format!("Usandolo{}: {what}", if d.reusable { " (non si consuma)" } else { " (si consuma)" }));
    }
    if !d.carried_stats.is_empty() {
        let what = d.carried_stats.iter().map(|(s, v)| format!("{} {}", stat(c, s), signed(*v))).collect::<Vec<_>>().join(", ");
        out.push(format!("Portandolo addosso: {what}"));
    }
    if d.victory_points > 0 {
        out.push(format!("Vale {} punti vittoria finché lo tiene la tua fazione", d.victory_points));
    }
    for col in c.collections.values().filter(|k| k.items.contains(&d.id) || k.tag.as_ref().is_some_and(|t| d.tags.contains(t))) {
        out.push(format!("Fa parte della collezione «{}» ({} punti a collezione completa)", col.name, col.victory_points));
    }
    if d.unique {
        out.push("Ne esiste uno solo al mondo".into());
    }
    if out.is_empty() {
        out.push("Non ha effetti diretti: si scambia, si vende o serve per produrre altro".into());
    }
    out
}

fn num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{v:.0}") } else { format!("{v:.1}") }
}

/// A requirement in words ("Grasso almeno 80", "membro della Chiesa", "ha la classe Prete"…).
pub fn condition(c: &Content, cond: &Condition) -> String {
    let name = |m: &str| m.to_string();
    match cond {
        Condition::Always => "nessun requisito".into(),
        Condition::Never => "impossibile".into(),
        Condition::All(v) => v.iter().map(|x| condition(c, x)).collect::<Vec<_>>().join(" e "),
        Condition::Any(v) => format!("({})", v.iter().map(|x| condition(c, x)).collect::<Vec<_>>().join(" oppure ")),
        Condition::Not(inner) => match inner.as_ref() {
            Condition::HasClass(k) => format!("non {}", c.classes.get(k).map_or(k.as_str(), |x| x.name.as_str())),
            Condition::HasStatusTag(t) => format!("nessun malanno di tipo {t}"),
            Condition::HasStatus(s) => format!("non {}", status(c, s)),
            Condition::IsRace(r) => format!("non {}", c.races.get(r).map_or(r.as_str(), |x| x.name.as_str())),
            Condition::MemberOf(f) => format!("non membro di {}", c.factions.get(f).map_or(f.as_str(), |x| x.name.as_str())),
            Condition::WantedAtLeast(_) => "non ricercato".into(),
            other => format!("non ({})", condition(c, other)),
        },
        Condition::StatAtLeast { stat: s, value } => format!("{} almeno {}", cap(&stat(c, s)), num(*value)),
        Condition::StatBelow { stat: s, value } if *value <= 1.0 => format!("{} a zero", cap(&stat(c, s))),
        Condition::StatBelow { stat: s, value } => format!("{} sotto {}", cap(&stat(c, s)), num(*value)),
        Condition::NeedBelow { need: n, value } => format!("{} sotto il {:.0}%", cap(&need(c, n)), value * 100.0),
        Condition::HasStatus(s) => format!("è {}", status(c, s)),
        Condition::HasStatusTag(t) => format!("ha un malanno di tipo {t}"),
        Condition::StatusTagCount { tag, count } => format!("almeno {count} status di tipo {tag} insieme"),
        Condition::HasTag(t) => format!("è {}", t.replace('_', " ")),
        Condition::HasItem { item: i, qty } if *qty > 1 => format!("possiede {qty}× {}", item(c, i)),
        Condition::HasItem { item: i, .. } => format!("possiede {}", item(c, i)),
        Condition::HasItemTag(t) => format!("possiede un oggetto di tipo {}", name(t).replace('_', " ")),
        Condition::MoneyAtLeast(m) => format!("almeno {m:.0} €"),
        Condition::InZone(z) => format!("si trova in {}", c.zone(z).map_or(z.as_str(), |x| x.name.as_str())),
        Condition::MemberOf(f) => format!("membro di {}", c.factions.get(f).map_or(f.as_str(), |x| x.name.as_str())),
        Condition::IsRace(r) => format!("razza {}", c.races.get(r).map_or(r.as_str(), |x| x.name.as_str())),
        Condition::HasClass(k) => format!("è {}", c.classes.get(k).map_or(k.as_str(), |x| x.name.as_str())),
        Condition::WantedAtLeast(v) => format!("ricercato almeno {}", num(*v)),
        Condition::Detained => "in arresto".into(),
        Condition::Disguised => "travestito".into(),
        Condition::HoldsTitle(t) => format!("è {}", c.titles.get(t).map_or(t.as_str(), |x| x.name.as_str())),
        Condition::HoldsAnyTitle => "ha un ruolo".into(),
        Condition::BondAtLeast { attraction, .. } if *attraction > 0.0 => format!("attrazione almeno {}", num(*attraction)),
        Condition::BondAtLeast { friendship, .. } => format!("amicizia almeno {}", num(*friendship)),
        Condition::Contest { stat: s, .. } => format!("vince il confronto di {}", stat(c, s)),
        Condition::IsSex(s) => match s {
            crate::stats::Sex::Male => "uomo".into(),
            crate::stats::Sex::Female => "donna".into(),
            crate::stats::Sex::NonBinary => "non binario".into(),
        },
        Condition::TitleVacant(t) => format!("il posto di {} è vacante", c.titles.get(t).map_or(t.as_str(), |x| x.name.as_str())),
        Condition::HpBelow(r) => format!("vita sotto il {:.0}%", r * 100.0),
        _ => "condizione speciale".into(),
    }
}

fn cap(s: &str) -> String {
    let mut ch = s.chars();
    ch.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + ch.as_str())
}

/// A requirement split into its top-level parts (one line each in the UI).
pub fn condition_parts(cond: &Condition) -> Vec<Condition> {
    match cond {
        Condition::All(v) => v.iter().flat_map(condition_parts).collect(),
        Condition::Always => vec![],
        other => vec![other.clone()],
    }
}
