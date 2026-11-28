module Workspace
  State = Data.define(:mode, :items, :runs, :repositories, :problems, :checklist, :push_repository, :preview) do
    def first_run?
      mode == :first_run
    end

    def placeholder?
      preview && !first_run?
    end

    def inbox_count
      items.size
    end

    def runs_count
      runs.size
    end

    def settings_warning?
      problems.any?
    end

    def banner_problem
      problems.first
    end

    def completed_steps
      checklist.count(&:done)
    end

    def current_step
      checklist.find(&:pending?)
    end

    def assign_issue_repository
      repositories.find(&:real)
    end
  end
end
