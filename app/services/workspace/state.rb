module Workspace
  State = Data.define(:mode, :items, :runs, :repositories, :problems, :preview) do
    def no_repositories?
      mode == :no_repositories
    end

    def placeholder?
      preview && !no_repositories?
    end

    def inbox_count
      items.size
    end

    def runs_count
      runs.size
    end

    def banner_problem
      problems.first
    end

    def assign_issue_repository
      repositories.find(&:real)
    end
  end
end
