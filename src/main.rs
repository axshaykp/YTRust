use regex::Regex;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashSet;
use warp::Filter;

// Struct to hold video data for JSON response
#[derive(Serialize)]
struct Video {
    video_id: String,
    img: String,
    title: String,
}

// Struct to hold both videos and shorts
#[derive(Serialize)]
struct VideoResponse {
    videos: Vec<Video>,
    shorts: Vec<Video>, // Add a field for shorts
}

#[tokio::main]
async fn main() {
    // Route to handle search requests with a query parameter
    let search_route = warp::path!("search" / String).and_then(handle_search);

    // Start the warp server
    warp::serve(search_route).run(([127, 0, 0, 1], 3030)).await;
}

// Function to handle the search request
async fn handle_search(query: String) -> Result<impl warp::Reply, warp::Rejection> {
    let url = format!("https://www.youtube.com/results?search_query={}", query);

    // Create an HTTP client and send the GET request
    let client = Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.152 Safari/537.36")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|_| warp::reject::not_found())?; // Simple error handling

    if !response.status().is_success() {
        return Err(warp::reject::not_found());
    }

    let html_text = response
        .text()
        .await
        .map_err(|_| warp::reject::not_found())?;

    // Define the regular expressions to extract videoId, thumbnail, and title
    let video_id_regex = Regex::new(r#""videoId":"([^"]+)""#).unwrap();
    let thumbnail_regex = Regex::new(r#"https://i\.ytimg\.com/vi/[^"]+\.jpg"#).unwrap(); // Matches full thumbnail URL
    let title_regex = Regex::new(r#""title":\{"runs":\[\{"text":"([^"]+)""#).unwrap(); // Fixed escaping

    let mut videos = Vec::new();
    let mut shorts = Vec::new(); // Create a separate vector for shorts
    let mut unique_video_ids = HashSet::new();

    // Iterate over matches and extract data
    let video_ids = video_id_regex.captures_iter(&html_text); // Use captures to get the group
    let thumbnails = thumbnail_regex.find_iter(&html_text);
    let titles = title_regex.captures_iter(&html_text); // Use captures to get the group

    for ((vid_match, thumb_match), title_match) in video_ids.zip(thumbnails).zip(titles) {
        // Extract the videoId (use the first capture group, which is inside the parentheses)
        let video_id = vid_match.get(1).map_or("", |m| m.as_str()).to_string();

        // Extract the thumbnail URL (use the full URL directly from the match)
        let thumbnail_url = thumb_match.as_str().to_string();

        // Extract the title (use the first capture group, which is inside the parentheses)
        let title = title_match.get(1).map_or("", |m| m.as_str()).to_string();

        if !unique_video_ids.contains(&video_id) {
            unique_video_ids.insert(video_id.clone());

            // Check if it's a short or regular video based on title length or other features
            if title.to_lowercase().contains("shorts") || title.len() < 30 {
                // Assume this is a short if "shorts" in title or title is short
                shorts.push(Video {
                    video_id,
                    img: thumbnail_url,
                    title,
                });
            } else {
                videos.push(Video {
                    video_id,
                    img: thumbnail_url,
                    title,
                });
            }
        }
    }

    // Return the response as JSON with both videos and shorts
    Ok(warp::reply::json(&VideoResponse { videos, shorts }))
}
