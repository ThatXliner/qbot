use std::time::Duration;
use serde_json::json;
fn main() {
 let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
 rt.block_on(async {
  let builder = reqwest::Client::builder().timeout(Duration::from_secs(20)).redirect(reqwest::redirect::Policy::none()).no_proxy();
  let builder = builder.resolve("dv-g3b-emudhra.we-have-a-problem.com", "15.235.19.186:443".parse().unwrap());
  let builder = builder.resolve("dv-g3b-aaa-emudhra.we-have-a-problem.com", "15.235.19.186:443".parse().unwrap());
  let builder = builder.resolve("example.com", "172.66.147.243:443".parse().unwrap());
  let client = builder.build().unwrap();
  for host in ["dv-g3b-emudhra.we-have-a-problem.com", "dv-g3b-aaa-emudhra.we-have-a-problem.com", "example.com"] {
   let url = format!("https://{host}/");
   let result = {
    match client.get(&url).send().await {
     Ok(r) => json!({"url":url,"status":r.status().as_u16(),"outcome":"accepted"}),
     Err(e) => json!({"url":url,"outcome":"error","error":format!("{e:?}")}),
    }
   };
   println!("{}",result);
  }
 });
}
