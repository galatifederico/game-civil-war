//! Demo: boots `sim_core` with the Fidenza world, runs 100 ticks and narrates the crime → arrest →
//! bribe → scoop → market chain. Then optionally keeps running with the Admin/UI API, `/metrics` and MCP.
//!
//! ```text
//! cargo run -p fidenza_world --release -- [--seed N] [--ticks N] [--serve [ADDR]] [--tick-ms N]
//!                                           [--mcp-stdio] [--compendium PATH] [--verbose]
//! ```

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sim_core::prelude::*;

struct Args {
    seed: u64,
    ticks: u64,
    serve: Option<String>,
    tick_ms: u64,
    mcp_stdio: bool,
    load: Option<String>,
    token: Option<String>,
    compendium: String,
    verbose: bool,
}

fn parse_args() -> Args {
    let mut a = Args { seed: 1, ticks: 100, serve: None, tick_ms: 1000, mcp_stdio: false, load: None, token: None, compendium: "compendium.json".into(), verbose: false };
    let mut it = std::env::args().skip(1).peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--seed" => a.seed = it.next().and_then(|v| v.parse().ok()).unwrap_or(a.seed),
            "--ticks" => a.ticks = it.next().and_then(|v| v.parse().ok()).unwrap_or(a.ticks),
            "--tick-ms" => a.tick_ms = it.next().and_then(|v| v.parse().ok()).unwrap_or(a.tick_ms),
            "--serve" => {
                let addr = match it.peek() {
                    Some(v) if !v.starts_with("--") => it.next().unwrap(),
                    _ => "127.0.0.1:8787".to_string(),
                };
                a.serve = Some(addr);
            }
            "--mcp-stdio" => a.mcp_stdio = true,
            "--load" => a.load = it.next(),
            "--token" => a.token = it.next(),
            "--compendium" => a.compendium = it.next().unwrap_or(a.compendium),
            "--verbose" | "-v" => a.verbose = true,
            "--help" | "-h" => {
                println!("uso: fidenza_world [--seed N] [--ticks N] [--serve [ADDR]] [--tick-ms N] [--mcp-stdio] [--load SAVE.json] [--token T] [--compendium PATH] [--verbose]");
                std::process::exit(0);
            }
            other => eprintln!("argomento ignorato: {other}"),
        }
    }
    a
}

/// Event kinds narrated during the demo.
const STORY: &[&str] = &[
    kind::CRIME,
    kind::CRIME_REPORTED,
    kind::SEARCH,
    kind::SEIZURE,
    kind::ARREST,
    kind::BRIBE,
    kind::BRIBE_FAILED,
    kind::RELEASE,
    kind::ARTICLE,
];

fn contraband_prices(sim: &Simulation) -> BTreeMap<String, f64> {
    let content = sim.content();
    let tag = content.bindings.contraband_tag.clone();
    sim.world
        .resource::<Market>()
        .items
        .iter()
        .filter(|(i, _)| content.items.get(*i).is_some_and(|d| d.tags.contains(&tag)))
        .map(|(i, m)| (content.items[i].name.clone(), m.price))
        .collect()
}

fn main() {
    let args = parse_args();
    // Logs on stderr, so that `--mcp-stdio` keeps stdout clean.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn,sim_core=info,fidenza_world=info".into()))
        .init();
    let metrics = if args.serve.is_some() { sim_core::server::install_metrics() } else { None };

    if let Some(path) = &args.load {
        let save = match Simulation::read_save(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Salvataggio illeggibile: {e}");
                std::process::exit(1);
            }
        };
        let sim = fidenza_world::builder(args.seed).and_then(|b| b.build_from_save(&save).map_err(|e| e.to_string()));
        match sim {
            Ok(sim) => {
                println!("Partita caricata da {path} (tick {})", sim.tick_count());
                if args.mcp_stdio {
                    return run_mcp_stdio(sim, args.tick_ms);
                }
                return run_server(sim, args.serve.as_deref().unwrap_or("127.0.0.1:8787"), args.tick_ms, metrics, args.token.clone(), args.seed);
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
    let mut sim = match fidenza_world::build(args.seed) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Errore nei contenuti: {e}");
            std::process::exit(1);
        }
    };
    if args.mcp_stdio {
        return run_mcp_stdio(sim, args.tick_ms);
    }

    let pawns = sim.snapshot(true).entities.iter().filter(|e| e.kind == "pawn").count();
    println!("══ sim_core + fidenza_world — seed {} ══", args.seed);
    println!("Contenuti: {} razze, {} classi, {} fazioni, {} oggetti, {} edifici; {} pedine in gioco.",
        sim.content().races.len(), sim.content().classes.len(), sim.content().factions.len(),
        sim.content().items.len(), sim.content().buildings.len(), pawns);

    let player_faction = "anarchici_commercio";
    let (thief_id, thief) = sim.find_template("scippatore").expect("lo scippatore della demo esiste");
    let thief_name = sim.world.get::<DisplayName>(thief).unwrap().0.clone();
    let treasury_before = sim.world.resource::<Factions>().treasury(player_faction);
    let prices_before = contraband_prices(&sim);
    println!("\nIl Leader degli Anarchici del Commercio (giocatore) ha {treasury_before:.0} euro nel Fondo di Gilda.");
    println!("{thief_name} gira per Piazza Garibaldi con le tasche vuote...\n");
    println!("── Cronaca ──");

    let mut bribed = false;
    let mut cursor = 0;
    for _ in 0..args.ticks {
        sim.tick();
        // The player reacts: as soon as their member is behind bars, the Leader pays the police.
        if !bribed && sim.world.get::<Detained>(thief).is_some() {
            bribed = true;
            let cost = sim_core::crime::bribe_cost(&sim.world, thief).unwrap_or(0.0);
            println!("[{:>3}] >>> Il Leader decide di pagare la tangente: la Polizia chiede {cost:.0} euro", sim.tick_count());
            sim.enqueue(SimCommand::Bribe { faction: player_faction.into(), target: thief_id, amount: None });
        }
        for e in sim.events().since(cursor) {
            let related = e.actor == Some(thief_id) || e.target == Some(thief_id) || e.kind == kind::ARTICLE;
            if args.verbose || (STORY.contains(&e.kind.as_str()) && related) {
                println!("[{:>3}] {:<15} {}", e.tick, e.kind, e.message);
            }
        }
        cursor = sim.events().last_id();
    }

    // ── Verification of the four demo points ──
    let ev = sim.events().all();
    let crime = ev.iter().find(|e| e.kind == kind::CRIME && e.actor == Some(thief_id));
    let reported = ev.iter().find(|e| e.kind == kind::CRIME_REPORTED && e.actor == Some(thief_id));
    let search = ev.iter().find(|e| e.kind == kind::SEARCH && e.target == Some(thief_id));
    let arrest = ev.iter().find(|e| e.kind == kind::ARREST && e.target == Some(thief_id));
    let bribe = ev.iter().find(|e| e.kind == kind::BRIBE && e.target == Some(thief_id));
    let feed = sim.world.resource::<Feed>().clone();
    let story_ids: Vec<u64> = ev
        .iter()
        .filter(|e| (e.actor == Some(thief_id) || e.target == Some(thief_id)) && e.newsworthiness > 0.0)
        .map(|e| e.id)
        .collect();
    let scoop = feed.articles.iter().find(|a| a.source_event.is_some_and(|s| story_ids.contains(&s)));
    let price_move = scoop.and_then(|a| ev.iter().find(|e| e.kind == kind::PRICE_CHANGE && e.tick >= a.tick));
    let wanted_now = sim.world.get::<Wanted>(thief).map_or(0.0, |w| w.level);

    let check = |ok: bool| if ok { "✔" } else { "✘" };
    println!("\n── Verifica ──");
    println!("{} 1. {} commette un crimine e il suo WantedLevel sale{}", check(crime.is_some() && reported.is_some()), thief_name,
        reported.and_then(|r| r.data.get("wanted")).map(|w| format!(" (a {w})")).unwrap_or_default());
    println!("{} 2. La Polizia Neutra interviene: perquisizione{} e arresto{}", check(search.is_some() && arrest.is_some()),
        search.map(|s| format!(" al tick {}", s.tick)).unwrap_or_default(), arrest.map(|a| format!(" al tick {}", a.tick)).unwrap_or_default());
    let treasury_after = sim.world.resource::<Factions>().treasury(player_faction);
    let recidivo = bribe.is_some_and(|b| ev.iter().any(|e| e.kind == kind::CRIME && e.actor == Some(thief_id) && e.tick > b.tick));
    println!("{} 3. Il Leader paga la tangente dal Fondo di Gilda e le accuse si azzerano{} (fondo {treasury_before:.0} → {treasury_after:.0}){}",
        check(bribe.is_some()),
        bribe.map(|b| format!(" al tick {}: {} euro", b.tick, b.data.get("amount").cloned().unwrap_or_default())).unwrap_or_default(),
        if recidivo { format!("; poi ci ricasca, ricercato ora {wanted_now:.0}") } else { format!("; ricercato ora {wanted_now:.0}") });
    println!("{} 4. Un Giornalista pubblica lo scoop su «{}»{} e i prezzi si muovono{}", check(scoop.is_some() && price_move.is_some()), feed.name,
        scoop.map(|a| format!(": «{}» ({})", a.headline, a.author_name)).unwrap_or_default(),
        price_move.map(|p| format!(": {}", p.message)).unwrap_or_default());

    println!("\n── Mercato del contrabbando ──");
    let prices_after = contraband_prices(&sim);
    for (name, before) in &prices_before {
        let after = prices_after.get(name).copied().unwrap_or(*before);
        println!("  {name:<20} {before:>7.2} → {after:>7.2}  ({:+.0}%)", (after - before) / before * 100.0);
    }

    println!("\n── {} (ultimi articoli) ──", feed.name);
    for a in feed.articles.iter().rev().take(8) {
        let tag = match a.truth {
            sim_core::content::Truth::Fake => "",
            sim_core::content::Truth::Propaganda => " [propaganda]",
            _ => "",
        };
        println!("  [{:>3}] «{}» — {}{tag}", a.tick, a.headline, a.author_name);
    }

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for e in ev {
        *counts.entry(e.kind.as_str()).or_default() += 1;
    }
    println!("\n── Il resto del mondo in {} tick ──", sim.tick_count());
    let show = ["death", "mutilation", "exposed", "transmutation", "infection", "ability", "shapeshift", "defection", "succession", "trigger", "tether_released", "masterpiece", "production", "job_done", "purchase", "payroll"];
    let line: Vec<String> = show.iter().filter_map(|k| counts.get(k).map(|n| format!("{k}: {n}"))).collect();
    println!("  {}", line.join(" · "));
    let snap = sim.snapshot(true);
    let mut scores: Vec<(&String, &i64)> = snap.scores.iter().collect();
    scores.sort_by(|a, b| b.1.cmp(a.1));
    println!("  Classifica punti vittoria: {}", scores.iter().take(5).map(|(f, s)| format!("{f} {s}")).collect::<Vec<_>>().join(", "));
    println!("  Hash dello stato (determinismo): {:016x}", sim.state_hash());

    match sim_core::compendium::write(sim.content(), sim.world.resource::<Params>(), &args.compendium) {
        Ok(()) => println!("\nCompendium scritto in {}", args.compendium),
        Err(e) => eprintln!("compendium non scritto: {e}"),
    }

    if let Some(addr) = args.serve {
        run_server(sim, &addr, args.tick_ms, metrics, args.token.clone(), args.seed);
    }
}

fn run_server(sim: Simulation, addr: &str, tick_ms: u64, metrics: Option<sim_core::server::PrometheusHandle>, token: Option<String>, seed: u64) {
    let addr: std::net::SocketAddr = addr.parse().expect("indirizzo non valido (es. 127.0.0.1:8787)");
    let state = sim_core::server::AppState {
        sim: Arc::new(Mutex::new(sim)),
        control: Arc::new(Mutex::new(sim_core::server::Control { paused: false, tick_ms })),
        metrics,
        ui_html: Some(Arc::new(fidenza_world::client_html().to_string())),
        factory: Some(Arc::new(move || fidenza_world::builder(seed))),
        token,
        assets: fidenza_world::sprites_dir(),
    };
    println!("\nServer attivo su http://{addr}  (admin: /admin/, client: /ui/, MCP: POST /mcp, metriche: /metrics) — Ctrl+C per uscire");
    let rt = tokio::runtime::Runtime::new().expect("runtime tokio");
    if let Err(e) = rt.block_on(sim_core::server::serve(state, addr)) {
        eprintln!("server: {e}");
    }
}

fn run_mcp_stdio(sim: Simulation, tick_ms: u64) {
    let shared = Arc::new(Mutex::new(sim));
    let ticker = shared.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(tick_ms.max(1)));
        ticker.lock().unwrap().tick();
    });
    if let Err(e) = sim_core::server::mcp::serve_stdio(shared) {
        eprintln!("mcp: {e}");
    }
}
