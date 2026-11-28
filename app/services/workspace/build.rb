module Workspace
  # Everything the sidebar and the home need for one request: real records where they
  # exist (user, repositories), placeholder data only for the requested preview.
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
        mode: no_repositories? ? :no_repositories : :inbox,
        items: PlaceholderData.items(scenario),
        runs: PlaceholderData.runs(scenario),
        repositories: no_repositories? ? [] : repositories,
        problems: problems,
        preview: scenario.present?
      )
    end

    private
    attr_reader :user, :scenario

    def no_repositories?
      scenario ? scenario == :no_repositories : real_repositories.empty?
    end

    def repositories
      entries = real_repositories
      entries = PlaceholderData.repositories.map { |entry_| entry_.with(username: user.username) } if entries.empty? && %i[inbox empty].include?(scenario)

      case scenario
      when :inbox then entries.map { |entry_| entry_.with(mirror: :ok) }
      when :empty then entries[0...-1].map { |entry_| entry_.with(mirror: :ok) } + [entries.last.with(mirror: :failing)]
      else entries
      end
    end

    def real_repositories
      @real_repositories ||= user.repositories.active.order(:name).map { |repository_| RepositoryEntry.new(name: repository_.name, slug: repository_.slug, username: user.username, mirror: :none, real: true) }
    end

    def problems
      scenario == :empty ? [Problem.new(kind: :mirror_failing, repository: repositories.last.name)] : []
    end
  end
end
