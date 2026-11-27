module Inbox
  # Placeholder data until runs, issues, pull requests and memory exist as models.
  # Delete this class when the Home reads real records.
  class Sample
    attr_reader :selected

    def initialize(selected_id = nil)
      @selected = decisions.find { |item_| item_.id == selected_id.to_s } || decisions.first
    end

    def decisions
      @decisions ||= [push_mirror_run, file_browser_run, bare_repositories_pr, https_clone_pr]
    end

    def working
      @working ||= [merge_commit_run]
    end

    def issues
      @issues ||= [
        Item.new(id: "issue_18", kind: :issue, state: :assigned, title: "Merge commit with per-repository lock", number: 18, details: { run_number: 32 }),
        Item.new(id: "issue_15", kind: :issue, state: :unassigned, title: "Per-commit diff view", number: 15),
        Item.new(id: "issue_23", kind: :issue, state: :unassigned, title: "Issues with title, description, state and comments", number: 23),
        Item.new(id: "issue_24", kind: :issue, state: :unassigned, title: "Pull requests between branches of the same repository", number: 24)
      ]
    end

    def repositories
      @repositories ||= [
        RepositoryEntry.new(name: "david/meelting", mirror: :in_sync, synced_at: 3.minutes.ago),
        RepositoryEntry.new(name: "david/veta", mirror: :failed, synced_at: 2.hours.ago)
      ]
    end

    def memory
      @memory ||= [
        MemoryEntry.new(path: "decisions/0004-bare-repos-on-disk.md", proposed_in: 8, touched_at: 5.hours.ago),
        MemoryEntry.new(path: "decisions/0003-merge-commit-only.md", proposed_in: nil, touched_at: 1.day.ago)
      ]
    end

    private

    def push_mirror_run
      Item.new(
        id: "run_31", kind: :run, state: :awaiting_approval, title: "Configurable push mirror to GitHub", number: 31,
        cost_cents: 184, duration_seconds: 702,
        details: {
          issue_number: 12, finished_at: 4.minutes.ago, files: 6, additions: 214, deletions: 9, commands: 14, retried: true,
          steps: [
            Step.new(kind: :command, text: "bin/rails test → 218 runs, 0 failures", seconds: 478),
            Step.new(kind: :command, text: "bin/rubocop → no offenses", seconds: 551),
            Step.new(kind: :command, text: "git commit -m \"✨ feat(mirrors): …\"", seconds: 680),
            Step.new(kind: :message, text: "Branch ready. Stopped before opening the PR.", seconds: 702)
          ],
          memory_files: %w[overview.md architecture.md decisions/0006-push-mirror-safeguard.md]
        }
      )
    end

    def file_browser_run
      Item.new(
        id: "run_30", kind: :run, state: :awaiting_approval, title: "File browser with syntax highlighting", number: 30,
        cost_cents: 271, duration_seconds: 1085,
        details: {
          issue_number: 14, finished_at: 22.minutes.ago, files: 9, additions: 388, deletions: 21, commands: 22, retried: false,
          steps: [
            Step.new(kind: :command, text: "bin/rails test → 224 runs, 0 failures", seconds: 940),
            Step.new(kind: :command, text: "bin/rubocop → no offenses", seconds: 1002),
            Step.new(kind: :command, text: "git commit -m \"✨ feat(files): browse files with syntax highlighting\"", seconds: 1071),
            Step.new(kind: :message, text: "Branch ready. Stopped before opening the PR.", seconds: 1085)
          ],
          memory_files: %w[overview.md conventions.md]
        }
      )
    end

    def bare_repositories_pr
      Item.new(
        id: "pr_8", kind: :pull_request, state: :ready_to_merge, title: "Create bare repositories on disk", number: 8,
        cost_cents: 96, duration_seconds: 451,
        details: { run_number: 28, tests: 214, additions: 133, deletions: 0, files: 4 }
      )
    end

    def https_clone_pr
      Item.new(
        id: "pr_9", kind: :pull_request, state: :conflicts, title: "Push and clone over HTTPS with a token", number: 9,
        cost_cents: 137, duration_seconds: 488,
        details: { run_number: 29, tests: 220, additions: 301, deletions: 12, files: 7, conflict_file: "config/routes.rb" }
      )
    end

    def merge_commit_run
      Item.new(
        id: "run_32", kind: :run, state: :running, title: "Merge commit with per-repository lock", number: 32,
        cost_cents: 62, duration_seconds: 258,
        details: { issue_number: 18, command: "bin/rails test test/models/git/merge_test.rb" }
      )
    end
  end
end
