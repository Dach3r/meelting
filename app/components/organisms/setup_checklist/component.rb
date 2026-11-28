module Organisms
  module SetupChecklist
    class Component < ViewComponent::Base
      RING_RADIUS = 150
      RING_CIRCUMFERENCE = (2 * Math::PI * RING_RADIUS).round(2)

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

      def ring_dash
        [RING_CIRCUMFERENCE * done_count / steps.size, 0.1].max.round(2)
      end

      def title_for(step)
        t("home.checklist.steps.#{step.key}.title", file: Workspace::Build::OVERVIEW_PATH)
      end
    end
  end
end
