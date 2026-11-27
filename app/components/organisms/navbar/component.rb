module Organisms
  module Navbar
    class Component < ViewComponent::Base
      def initialize(authenticated:)
        @authenticated = authenticated
      end

      private
      attr_reader :authenticated

      def tone
        authenticated ? :light : :deep
      end

      def nav_classes
        if authenticated
          "fixed top-0 inset-x-0 z-20 border-b-nb border-ink bg-paper/90 backdrop-blur"
        else
          "fixed top-0 inset-x-0 z-20 border-b-nb border-brand-300/20 bg-ink"
        end
      end
    end
  end
end
