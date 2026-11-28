module Organisms
  module SetupChecklist
    class Component < ViewComponent::Base
      RING_RADIUS = 150
      RING_CIRCUMFERENCE = (2 * Math::PI * RING_RADIUS).round(2)
      ACTIONS = { access_token: :new_access_token_path, repository: :new_repository_path }.freeze

      def initialize(steps:)
        @steps = steps
      end

      private
      attr_reader :steps

      def done_count
        steps.count(&:done)
      end

      def current
        steps.find { |step_| !step_.done }
      end

      def action_path(step)
        helper = ACTIONS[step.key]
        send(helper) if helper
      end

      def ring_dash
        [RING_CIRCUMFERENCE * done_count / steps.size, 0.1].max.round(2)
      end

      def title_for(step)
        t("home.checklist.steps.#{step.key}.title", file: ".project/overview.md")
      end
    end
  end
end
