module Molecules
  module NavItem
    class Component < ViewComponent::Base
      renders_one :trailing

      def initialize(label:, href:, icon:, current: false, muted: false, compact: false)
        @label = label
        @href = href
        @icon = icon
        @current = current
        @muted = muted
        @compact = compact
      end

      private
      attr_reader :label, :href, :icon, :current, :muted, :compact

      def classes
        class_names(
          "flex items-center gap-3.5 rounded-xl px-4 text-[15px] font-semibold",
          compact ? "h-[46px]" : "h-[50px]",
          current ? "bg-forest-active text-lime" : (muted ? "text-forest-muted hover:bg-forest-active/50" : "text-forest-soft hover:bg-forest-active/50")
        )
      end

      def icon_classes
        class_names("size-[22px] shrink-0", current ? "text-lime" : "text-forest-muted")
      end
    end
  end
end
