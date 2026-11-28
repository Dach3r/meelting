module Workspace
  # Placeholder data for the navigation shell and the home, until issues, pull requests
  # and runs exist as models (Phases 2 and 3). Only reachable through ?preview in development.
  # Delete each part when its phase brings the real records.
  module PlaceholderData
    SCENARIOS = %i[inbox empty no_repositories].freeze

    ITEMS = [
      InboxItem.new(kind: :approve, title: "#14 Configurable push mirror to GitHub", repository: "meelting", agent: "claude-code", waiting_seconds: 12.minutes.to_i, target: :run),
      InboxItem.new(kind: :merge, title: "!9 File browser with syntax highlighting", repository: "meelting", agent: "claude-code", waiting_seconds: 2.hours.to_i, flag: :modifies_project, target: :pull_request),
      InboxItem.new(kind: :failed, title: "#16 Scoped tokens for sandbox clone", repository: "meelting", agent: "claude-code", waiting_seconds: 5.hours.to_i, target: :run)
    ].freeze

    RUNS = [
      RunEntry.new(title: "#18 Merge commit with per-repository lock", last_event: "bin/rails test test/models/git/merge_test.rb", elapsed_seconds: 258)
    ].freeze

    REPOSITORIES = [
      RepositoryEntry.new(name: "meelting", slug: "meelting", username: nil, mirror: :ok, real: false),
      RepositoryEntry.new(name: "veta", slug: "veta", username: nil, mirror: :ok, real: false)
    ].freeze

    def self.items(scenario)
      scenario == :inbox ? ITEMS.sort_by { |item_| -item_.waiting_seconds } : []
    end

    def self.runs(scenario)
      scenario == :inbox ? RUNS : []
    end

    def self.repositories
      REPOSITORIES
    end
  end
end
