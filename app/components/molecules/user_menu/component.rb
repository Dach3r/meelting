module Molecules
  module UserMenu
    class Component < ViewComponent::Base
      def initialize(user:, compact: false)
        @user = user
        @compact = compact
      end

      private
      attr_reader :user, :compact

      def initial
        user.initials.first
      end

      def summary_classes
        class_names(
          "flex cursor-pointer list-none items-center rounded-xl [&::-webkit-details-marker]:hidden",
          compact ? "size-[50px] justify-center hover:bg-forest-active/50" : "h-[54px] gap-3.5 pr-4 pl-[9px] text-white hover:bg-forest-active/50"
        )
      end

      def panel_classes
        class_names(
          "absolute z-40 grid w-56 gap-1 rounded-2xl border border-cream-line bg-white p-2",
          compact ? "bottom-0 left-full ml-3" : "bottom-full left-0 mb-2"
        )
      end
    end
  end
end
