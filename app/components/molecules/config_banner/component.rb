module Molecules
  module ConfigBanner
    class Component < ViewComponent::Base
      def initialize(problem:)
        @problem = problem
      end

      private
      attr_reader :problem

      def text(key)
        t("home.banner.#{problem.kind}.#{key}", repository: problem.repository)
      end
    end
  end
end
