module Organisms
  module InProgress
    class Component < ViewComponent::Base
      LIMIT = 3

      def initialize(runs:, preview:)
        @runs = runs
        @preview = preview
      end

      private
      attr_reader :runs, :preview

      def visible_runs
        runs.first(LIMIT)
      end
    end
  end
end
