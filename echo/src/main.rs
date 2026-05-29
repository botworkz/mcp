#[tokio::main]
async fn main() -> anyhow::Result<()> {
    botwork_mcp_echo::run().await
}
