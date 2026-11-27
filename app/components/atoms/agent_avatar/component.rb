module Atoms
  module AgentAvatar
    class Component < ViewComponent::Base
      SIZES = {
        sm: { box: "size-6 rounded-lg", icon: "size-3.5" },
        md: { box: "size-9 rounded-xl", icon: "size-[18px]" },
        lg: { box: "size-[52px] rounded-2xl", icon: "size-6" }
      }.freeze

      def initialize(size: :md)
        @size = SIZES.fetch(size)
      end

      def call
        tag.span class: class_names("inline-flex shrink-0 items-center justify-center bg-graphite-900 text-sun-400", @size[:box]), "aria-hidden": true do
          render Atoms::Icon::Component.new(name: :bot, class: @size[:icon])
        end
      end
    end
  end
end
