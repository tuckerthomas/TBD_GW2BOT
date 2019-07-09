extern crate serde;
extern crate serde_json;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::fmt;

use serde::Deserialize;

use serenity::{
    http::AttachmentType,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

struct Handler;

#[derive(Deserialize, Debug)]
struct DiscordConfig {
    client_id: u64,
    client_secret: String,
    bot_token: String,
}

// Multiple Error Type Handling
enum BotError {
    ReqwestError(reqwest::Error),
    SerdeError(serde_json::Error)
}

impl From<reqwest::Error> for BotError {
    fn from(error: reqwest::Error) -> Self {
        BotError::ReqwestError(error)
    }
}

impl From<serde_json::Error> for BotError {
    fn from(error: serde_json::Error) -> Self {
        BotError::SerdeError(error)
    }
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       match *self {
           BotError::ReqwestError(ref err) => write!(f, "Reqwest Error: {}", err),
           BotError::SerdeError(ref err) => write!(f, "Serde Error: {}", err)
       } 
    }
}

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!hello" {
            // The create message builder allows you to easily create embeds and messages
            // using a builder syntax.
            // This example will create a message that says "Hello, World!", with an embed that has
            // a title, description, three fields, and a footer.
            let msg = msg.channel_id.send_message(&ctx.http, |m| {
                m.content("Hello, World!");
                m.embed(|e| {
                    e.title("This is a title");
                    e.description("This is a description");
                    e.image("attachment://ferris_eyes.png");
                    e.fields(vec![
                        ("This is the first field", "This is a field body", true),
                        (
                            "This is the second field",
                            "Both of these fields are inline",
                            true,
                        ),
                    ]);
                    e.field(
                        "This is the third field",
                        "This is not an inline field",
                        false,
                    );
                    e.footer(|f| {
                        f.text("This is a footer");

                        f
                    });
                    e
                });
                m.add_file(AttachmentType::Path(Path::new("./ferris_eyes.png")));
                m
            });

            if let Err(why) = msg {
                println!("Error sending message: {:?}", why);
            }
        }
        
        if msg.content == "!fractals" {
            let dailies_res = get_dailies();

            match dailies_res {
                Ok(dailies) => {
                    // We got the dailies, lets get the achievements
                    let achievements = get_achievements(dailies);
                    
                    match achievements {
                        Ok(achievements) => {
                            // 0, 1, 2 -> Recs
                            // 14, 10, 6 -> Daily
                            let msg = msg.channel_id.send_message(&ctx.http, |m| {
                                m.embed(|emb| {
                                    emb.title("Tomorrow's Daily Fractals:");
                                    let daily_frac_string = format!("{}, {}, {}", achievements[6].name, achievements[10].name, achievements[14].name);
                                    let rec_frac_string = format!("{}, {}, {}", achievements[0].name, achievements[1].name, achievements[2].name);
                                    emb.field("Daily Fractals", daily_frac_string, false);
                                    emb.field("Recommended Fractals", rec_frac_string, false);
                                    emb
                                });
                                m
                            });
                            if let Err(msg_err) = msg {
                                println!("Error sending message: {:?}", msg_err);
                            }
                        },
                        Err(e) => { // Achievements Error
                            eprintln!("Could Not Get Dailies: {}", e);
                            let msg = msg.channel_id.send_message(&ctx.http, |m| {
                                m.embed(|emb| {
                                    emb.title("Error");
                                    emb.description(&e);
                                    emb
                                });
                                m
                            });
                            if let Err(msg_err) = msg {
                                println!("Error sending message: {:?}", msg_err);
                            }
                            
                        }
                    }
                },
                Err(e) => { // Dailies Error
                    eprintln!("Could Not Get Dailies: {}", e);
                    let msg = msg.channel_id.send_message(&ctx.http, |m| {
                        m.embed(|emb| {
                            emb.title("Error");
                            emb.description(&e);
                            emb
                        });
                        m
                    });
                    if let Err(msg_err) = msg {
                        println!("Error sending message: {:?}", msg_err);
                    }
                },
            }
        }
    }

    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn main() {
    let discord_config = read_discord_config("config.toml".to_owned()).unwrap(); //TODO: handle errors
    println!("Client ID: {}", discord_config.client_id);

    let mut client = Client::new(&discord_config.bot_token, Handler).expect("Err creating client");

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}

fn get_dailies() -> Result<Dailies, BotError>{
    let uri = "http://api.guildwars2.com/v2/achievements/daily/tomorrow";

    let body = reqwest::get(uri)?
        .text()?;

    let json = serde_json::from_str(&body)?;
    Ok(json)
}

fn get_achievements(dailies: Dailies) -> Result<Vec<Achievement>, BotError> {

    // TODO: Simplify url concat
    let uri = "https://api.guildwars2.com/v2/achievements?ids=".to_owned();

    let mut uri_mut = uri;

    for daily in dailies.fractals {
        let daily_id = format!("{},", daily.id);
        uri_mut.push_str(&daily_id);
    }

    uri_mut.pop(); // remove last ',' from above

    println!("Uri Requested: {:#?}", uri_mut);
    let body = reqwest::get(uri_mut.as_str())?
        .text()?;
    let achievements = serde_json::from_str(&body)?;
    Ok(achievements)
}

fn read_discord_config(filename: String) -> std::io::Result<DiscordConfig> {
    let file = File::open(filename)?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let config: DiscordConfig = toml::from_str(contents.as_str()).unwrap();
    Ok(config)
}

#[derive(Deserialize, Debug)]
enum Expansions {
    GuildWars2,
    HeartOfThorns,
    PathOfFire,
}

#[derive(Deserialize, Debug)]
struct Level {
    min: i32,
    max: i32,
}

#[derive(Deserialize, Debug)]
struct Daily {
    id: i32,
    level: Level,
    required_access: Vec<Expansions>,
}

#[derive(Deserialize, Debug)]
struct Dailies {
    pve: Vec<Daily>,
    pvp: Vec<Daily>,
    wvw: Vec<Daily>,
    fractals: Vec<Daily>,
}

#[derive(Deserialize, Debug)]
struct Tier {
    count: i32,
    points: i32,
}

#[derive(Deserialize, Debug)]
struct Item {
    r#type: String,
    id: i32,
    count: i32,
}

#[derive(Deserialize, Debug)]
struct Achievement {
    id: i32,
    icon: String,
    name: String,
    description: String,
    requirement: String,
    locked_text: String,
    r#type: String,
    flags: Vec<String>,
    tiers: Vec<Tier>,
    rewards: Vec<Item>,
}