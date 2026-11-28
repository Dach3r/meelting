module Organisms
  module SetupChecklist
    class Component < ViewComponent::Base
      def initialize(steps:, repository: nil)
        @steps = steps
        @repository = repository
      end

      private
      attr_reader :steps, :repository

      def done_count
        steps.count(&:done)
      end

      def current
        steps.find(&:pending?)
      end

      def title_for(step)
        t("home.checklist.steps.#{step.key}.title", file: Workspace::Build::OVERVIEW_PATH)
      end
    end
  end
end
