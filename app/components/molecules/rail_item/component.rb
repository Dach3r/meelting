module Molecules
  module RailItem
    class Component < ViewComponent::Base
      renders_one :badge

      def initialize(label:, href:, icon:, current: false)
        @label = label
        @href = href
        @icon = icon
        @current = current
      end

      def call
        link_to href, class: classes, aria: { label: label, current: (current ? "page" : nil) } do
          safe_join([render(Atoms::Icon::Component.new(name: icon, class: "size-[22px]", "stroke-width": 1.75)), badge].compact)
        end
      end

      private
      attr_reader :label, :href, :icon, :current

      def classes
        class_names(
          "relative flex h-[50px] w-full items-center justify-center rounded-xl",
          current ? "bg-forest-active text-lime" : "text-forest-muted hover:bg-forest-active/50"
        )
      end
    end
  end
end
