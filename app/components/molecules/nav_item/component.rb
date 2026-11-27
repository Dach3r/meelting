module Molecules
  module NavItem
    class Component < ViewComponent::Base
      renders_one :trailing

      def initialize(label:, href:, icon:, current: false, muted: false)
        @label = label
        @href = href
        @icon = icon
        @current = current
        @muted = muted
      end

      private
      attr_reader :label, :href, :icon, :current, :muted

      def classes
        class_names(
          "flex items-center gap-2.5 rounded-full px-3.5 py-2.5 font-medium hover:bg-white/60",
          current ? "bg-white font-semibold text-graphite-900" : (muted ? "text-graphite-500" : "text-graphite-700")
        )
      end
    end
  end
end
