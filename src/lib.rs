pub use struktur::*;
pub use yang_tau_tau_aja::*;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
const JSON_POINTER: &str = "/contents/twoColumnSearchResultsRenderer/primaryContents/sectionListRenderer/contents/0/itemSectionRenderer/contents";
const SEARCH_URL: &str = "https://www.youtube.com/results?search_query=";
const WATCH_URL: &str = "https://www.youtube.com/watch?v=";

pub mod struktur {
  use ratatui::widgets::ListState;
  use serde::Deserialize;

  #[derive(Debug, Default, Deserialize)]
  pub struct YoutubeVideoInfo {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration: String,
    pub avatar: String,
    pub url: String,
    pub thumbnail: String,
    pub viewers: String
  }

  pub struct App {
    pub videos: Vec<YoutubeVideoInfo>,
    pub selected: usize,
    pub query: String,
    pub state: ListState,
    pub message: String
  }
}
pub mod yang_tau_tau_aja {

  use reqwest::Client;
  use scraper::{Html, Selector};
  use crate::{JSON_POINTER, SEARCH_URL, USER_AGENT, WATCH_URL, YoutubeVideoInfo};
  
  pub async fn search_youtube_videos(query: &str) -> Result<Vec<YoutubeVideoInfo>, Box<dyn std::error::Error>> {
    let client = Client::builder()
      .user_agent(USER_AGENT)
      .build()?;
    
    let res = client.get(&format!("{}{}", SEARCH_URL, query))
      .header("accept-language", "en-US,en;q=0.9")
      .send().await?.text().await?;
  
    let document = Html::parse_document(&res);
    let selector = Selector::parse("script").unwrap();

    let mut results = Vec::<YoutubeVideoInfo>::new();

    for script in document.select(&selector) {
      let text = script.text().collect::<String>();
      if text.contains("ytInitialData") {
        if let Some(start) = text.find("var ytInitialData = ") {
          let json_part = &text[start + 20..];
          if let Some(end) = json_part.find(";") {
            let json_str = &json_part[..end];
            let json = serde_json::from_str::<serde_json::Value>(json_str)?;
  
            if let Some(contents) = json.pointer(JSON_POINTER) {
              for item in contents.as_array().unwrap() {
                if let Some(video) = item.get("videoRenderer") {
                  let id = video["videoId"].as_str().unwrap_or("");
                  let video_info = YoutubeVideoInfo {
                    id: id.into(),
                    title: video["title"]["runs"][0]["text"].as_str().unwrap_or("").into(),
                    channel: video["ownerText"]["runs"][0]["text"].as_str().unwrap_or("").into(),
                    duration: video["lengthText"]["simpleText"].as_str().unwrap_or("").into(),
                    avatar: video["avatar"]["decoratedAvatarViewModel"]["avatar"]["avatarViewModel"]["image"]["sources"][0]["url"].as_str().unwrap_or("").into(),
                    url: format!("{}{}", WATCH_URL, id),
                    thumbnail: video["thumbnail"]["thumbnails"][1]["url"].as_str().unwrap_or("").into(),
                    viewers: video["viewCountText"]["simpleText"].as_str().unwrap_or("").into()
                  };
                  results.push(video_info);
                }
              }
            }
          }
        }
      }
    }
    Ok(results)
  }
}

pub mod tui {
  use ratatui::{crossterm::event::{self, Event, KeyCode}, prelude::*, widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph}};
  use crate::{App, YoutubeVideoInfo, search_youtube_videos};

  impl App {
    pub fn new() -> Self {
      let mut states = ListState::default();
      states.select(Some(0));

      Self {
        videos: vec![],
        selected: 0,
        query: String::new(),
        state: states,
        message: "Ketik query lalu enter".to_string()
      }
    }
  }

  pub async fn start_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> std::io::Result<()> {
    loop {
      terminal.draw(|frame| {
        let size = frame.area();
        let chunks = Layout::default()
          .direction(Direction::Vertical)
          .margin(2)
          .constraints([
            Constraint::Min(5),
            Constraint::Length(12),
            Constraint::Length(3)
          ])
          .split(size);

        let items= if app.videos.is_empty() {
          vec![ListItem::new(app.message.clone())]
        } else {
          app.videos.iter().enumerate().map(|(index, video)| {
            let style = if index == app.selected {
              Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
              Style::default()
            };
            ListItem::new(format!("({}). {}", index, video.title)).style(style)
          }).collect()
        };

        let list = List::new(items)
          .block(Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(2, 2, 1, 1))
            .title(" Youtube Search ")
          ).highlight_style(
            Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
          ).highlight_symbol("> ");
        frame.render_stateful_widget(list, chunks[0], &mut app.state);

        let selected_info = if !app.videos.is_empty() {
          let video = &app.videos[app.selected];
          vec![
            ListItem::new(format!("ID: {}", video.id)),
            ListItem::new(format!("Title: {}", video.title)),
            ListItem::new(format!("Channel: {}", video.channel)),
            ListItem::new(format!("Views: {}", video.viewers)),
            ListItem::new(format!("Duration: {}", video.duration)),
            ListItem::new(format!("URL: {}", video.url)),
            ListItem::new(format!("Avatar: {}", video.avatar)),
            ListItem::new(format!("Thumbnail: {}", video.thumbnail))
          ]
        } else {
          vec![ListItem::new("Pencet 0 atau Esc untuk keluar".to_string())]
        };

        let info = List::new(selected_info)
          .style(Style::default().fg(Color::Green))
          .block(Block::default().borders(Borders::ALL).title(" Youtube Info ").padding(Padding::new(2, 2, 1, 1))
        );

        frame.render_widget(info, chunks[1]);

        let input = Paragraph::new(&*app.query)
          .style(Style::default()).fg(Color::Red)
          .block(Block::default().borders(Borders::ALL).padding(Padding::horizontal(2)).title(" Masukan Query "));
        frame.render_widget(input, chunks[2]);
      })?;

      if event::poll(std::time::Duration::from_millis(200))? {
        if let Event::Key(key) = event::read()? {
          match key.code {
            KeyCode::Char('0') | KeyCode::Esc => return Ok(()),
            KeyCode::Char(c) => app.query.push(c),
            KeyCode::Backspace => {
              app.query.pop();
            },
            KeyCode::Enter => {
              if !app.query.trim().is_empty() {
                let query = app.query.trim().to_string();
                app.message = format!("Mencari: {}...", query);
                terminal.draw(|frame| {
                  let size = frame.area();
                  let block = Block::default().borders(Borders::ALL).padding(Padding::new(2, 2, 2, 2)).title(Line::from(" Mencari... ").centered());
                  frame.render_widget(block, size);
                })?;
                match search_youtube_videos(&query).await {
                  Ok(video) => {
                    app.videos = video;
                    app.selected = 0;
                    app.message = format!("Hasil untuk: {}", query);
                  }
                  Err(e) => {
                    app.videos = vec![YoutubeVideoInfo::default()];
                    app.message = e.to_string();
                  }
                }
                app.query.clear();
              } else {
                let video = &app.videos[app.selected];
                app.message = video.url.clone();
              }
            }
            KeyCode::Up => {
              if app.selected > 0 {
                app.selected -= 1;
                app.state.select(Some(app.selected));
              }
            }
            KeyCode::Down => {
              if app.selected < app.videos.len().saturating_sub(1) {
                app.selected += 1;
                app.state.select(Some(app.selected));
              }
            }
            _ => {}
          }
        }
      }
    }
  }
}