module Workspace
  # Everything the sidebar and the home need for one request: real records where they
  # exist (user, tokens, repositories), placeholder data only for the requested preview.
  class Build
    def self.call(user:, scenario: nil)
      new(user: user, scenario: scenario).call
    end

    def initialize(user:, scenario:)
      @user = user
      @scenario = scenario
    end

    def call
      State.new(
        mode: first_run? ? :first_run : :inbox,
        items: PlaceholderData.items(scenario),
        runs: PlaceholderData.runs(scenario),
        repositories: repositories,
        problems: problems,
        checklist: checklist,
        preview: scenario.present?
      )
    end

    private
    attr_reader :user, :scenario

    def first_run?
      scenario ? scenario == :first_run : checklist.any? { |step_| !step_.done }
    end

    def checklist
      @checklist ||= [
        ChecklistStep.new(key: :access_token, done: user.access_tokens.active.exists?),
        ChecklistStep.new(key: :repository, done: user.repositories.active.where.not(pushed_at: nil).exists?),
        ChecklistStep.new(key: :mirror, done: false),
        ChecklistStep.new(key: :model_key, done: false),
        ChecklistStep.new(key: :overview, done: false),
        ChecklistStep.new(key: :first_issue, done: false)
      ]
    end

    def repositories
      entries = real_repositories
      entries = PlaceholderData.repositories if entries.empty? && %i[inbox empty].include?(scenario)

      case scenario
      when :inbox then entries.map { |entry_| entry_.with(mirror: :ok) }
      when :empty then entries[0...-1].map { |entry_| entry_.with(mirror: :ok) } + [entries.last.with(mirror: :failing)]
      else entries
      end
    end

    def real_repositories
      user.repositories.active.order(:name).map { |repository_| RepositoryEntry.new(name: repository_.name, slug: repository_.slug, mirror: :none, real: true) }
    end

    def problems
      case scenario
      when :inbox then []
      when :empty then [Problem.new(kind: :mirror_failing, repository: repositories.last.name)]
      when :first_run, nil then [Problem.new(kind: :missing_model_key)]
      end
    end
  end
end
