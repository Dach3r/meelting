module Workspace
  # Everything the sidebar and the home need for one request: real records where they
  # exist (user, tokens, repositories), placeholder data only for the requested preview.
  class Build
    OVERVIEW_PATH = ".project/overview.md".freeze

    def self.call(user:, scenario: nil)
      new(user: user, scenario: scenario).call
    end

    def initialize(user:, scenario:)
      @user = user
      @scenario = scenario
    end

    def call
      user.complete_onboarding! if scenario.nil? && !user.onboarding_completed? && checklist.all?(&:done)

      State.new(
        mode: first_run? ? :first_run : :inbox,
        items: PlaceholderData.items(scenario),
        runs: PlaceholderData.runs(scenario),
        repositories: repositories,
        problems: problems,
        checklist: first_run? ? checklist : [],
        push_repository: push_repository,
        preview: scenario.present?
      )
    end

    private
    attr_reader :user, :scenario

    def first_run?
      scenario ? scenario == :first_run : !user.onboarding_completed?
    end

    def checklist
      @checklist ||= [
        ChecklistStep.new(key: :repository, done: pushed_repositories.any?),
        ChecklistStep.new(key: :model_key, done: false),
        ChecklistStep.new(key: :overview, done: pushed_repositories.any? { |repository_| repository_.file_on_default_branch?(OVERVIEW_PATH) }),
        ChecklistStep.new(key: :first_issue, done: false, available: false)
      ]
    end

    def pushed_repositories
      @pushed_repositories ||= user.repositories.active.where.not(pushed_at: nil).order(:created_at).to_a
    end

    def push_repository
      user.repositories.active.order(:created_at).first if first_run?
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
      when :first_run then []
      when nil then first_run? ? [] : [Problem.new(kind: :missing_model_key)]
      end
    end
  end
end
