cask "agent-scope" do
  version "0.2.0"
  sha256 "PLACEHOLDER_REPLACE_AFTER_RELEASE"

  url "https://github.com/NejaTive/agent-scope/releases/download/v#{version}/Agent%20Scope_#{version}_aarch64.dmg"
  name "Agent Scope"
  desc "Read-only viewer for Claude Code and Cursor configs"
  homepage "https://github.com/NejaTive/agent-scope"

  depends_on macos: ">= :big_sur"
  depends_on arch: :arm64

  app "Agent Scope.app"

  zap trash: [
    "~/Library/Application Support/com.agentscope.desktop",
    "~/Library/Caches/com.agentscope.desktop",
    "~/Library/Preferences/com.agentscope.desktop.plist",
    "~/Library/Saved Application State/com.agentscope.desktop.savedState",
  ]
end
