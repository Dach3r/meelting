module Atoms
  module AgentAvatar
    class Component < ViewComponent::Base
      SIZES = {
        sm: { box: "size-6 rounded-lg", icon: "size-3.5" },
        md: { box: "size-9 rounded-[10px]", icon: "size-[18px]" },
        lg: { box: "size-11 rounded-xl", icon: "size-5" }
      }.freeze

      def initialize(size: :md)
        @size = SIZES.fetch(size)
      end

      def call
        tag.span class: class_names("inline-flex shrink-0 items-center justify-center bg-lime text-forest", @size[:box]), "aria-hidden": true do
          render Atoms::Icon::Component.new(name: :bot, class: @size[:icon], "stroke-width": 2)
        end
      end
    end
  end
end
