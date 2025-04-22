use gloo::file::FileList;
use leptos::*;
use serde::Deserialize;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement};

#[derive(Clone, Deserialize)]
struct HdrResponse {
    message: String,
    download_url: String,
}

#[component]
fn App() -> impl IntoView {
    let (files, set_files) = create_signal::<Option<FileList>>(None);
    let (uploading, set_uploading) = create_signal(false);
    let (response, set_response) = create_signal::<Option<HdrResponse>>(None);
    let (error, set_error) = create_signal::<Option<String>>(None);

    let handle_change = move |ev: Event| {
        ev.prevent_default();
        let input = event_target::<HtmlInputElement>(&ev);
        if let Some(files_list) = input.files() {
            let files = FileList::from(files_list);
            set_files.set(Some(files));
        }
    };

    let upload = move |_| {
        set_uploading.set(true);
        set_response.set(None);
        set_error.set(None);
        println!("Uploading files...");
        if let Some(files) = files.get() {
            spawn_local(async move {
                let mut form = reqwest::multipart::Form::new();

                for file in files.iter() {
                    // Create a part from the file
                    let file_name = file.name();
                    let file_bytes = gloo::file::futures::read_as_bytes(&file).await.unwrap();
                    let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name.clone());

                    // Add the part to the form
                    form = form.part(file_name, part);
                }

                match reqwest::Client::new()
                    .post("http://100.90.241.174:8080/upload")
                    .multipart(form)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            match resp.json::<HdrResponse>().await {
                                Ok(data) => set_response.set(Some(data)),
                                Err(e) => {
                                    set_error.set(Some(format!("Error parsing response: {}", e)))
                                }
                            }
                        } else {
                            set_error.set(Some(format!("Error: {}", resp.status())));
                        }
                    }
                    Err(e) => set_error.set(Some(format!("Request error: {}", e))),
                }

                set_uploading.set(false);
            });
        } else {
            set_error.set(Some("Please select files to upload".to_string()));
            set_uploading.set(false);
        }
    };

    view! {
        <div class="container">
            <h1>"ORF Image HDR Merger"</h1>

            <div class="upload-section">
                <label for="file-upload" class="custom-file-upload">
                    "Select ORF images"
                </label>
                <input
                    id="file-upload"
                    type="file"
                    accept=".orf,.ORF"
                    multiple=true
                    on:change=handle_change
                    disabled=move || uploading.get()
                />

                <div class="selected-files">
                    {move || {
                        files.get().map(|files| {
                            if files.iter().count() > 0 {
                                view! {
                                    <>
                                    <p>
                                        "Selected " {files.iter().count()} " files"
                                    </p>
                                    <ul>
                                        {files.iter().map(|file| {
                                            view! { <li>{file.name()}</li> }
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                    </>
                                }
                            } else {
                                view! { <><p>"No files selected"</p></> }
                            }
                        })
                    }}
                </div>

                <button
                    on:click=upload
                    disabled=move || uploading.get()
                >
                    {move || {
                        if uploading.get() {
                            "Uploading..."
                        } else {
                            "Upload and Merge"
                        }
                    }}
                </button>
            </div>

            {move || {
                error.get().map(|err| {
                    view! {
                        <div class="error">
                            <p>{err}</p>
                        </div>
                    }
                })
            }}

            {move || {
                response.get().map(|resp| {
                    view! {
                        <div class="result">
                            <p>{resp.message.clone()}</p>
                            <a
                                href=format!("http://100.90.241.174:8080{}", resp.download_url)
                                download=true
                                target="_blank"
                                class="download-button"
                            >
                                "Download Merged DNG"
                            </a>
                        </div>
                    }
                })
            }}
        </div>
    }
}

fn event_target<T: JsCast>(event: &Event) -> T {
    event.target().unwrap().dyn_into::<T>().unwrap()
}

fn main() {
    mount_to_body(|| view! { <App /> })
}
