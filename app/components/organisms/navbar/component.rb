module Organisms
  module Navbar
    class Component < ViewComponent::Base
      def initialize(authenticated:, variant: :bar)
        @authenticated = authenticated
        @variant = variant
      end

      private
      attr_reader :authenticated, :variant

      def floating?
        variant == :floating
      end

      def tone
        floating? ? :deep : :light
      end

      def nav_classes
        if floating?
          "relative z-10 rounded-ui border border-deep-line bg-deep-surface backdrop-blur"
        else
          "fixed top-0 inset-x-0 z-10 border-b border-line bg-canvas/90 backdrop-blur"
        end
      end

      def inner_classes
        if floating?
          "flex h-12 items-center justify-between px-4"
        else
          "mx-auto flex h-16 max-w-6xl items-center justify-between px-5"
        end
      end
    end
  end
end
