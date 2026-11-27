module Atoms
  module StatusChip
    class Component < ViewComponent::Base
      TONES = {
        neutral: "bg-graphite-50 text-graphite-700",
        wait: "bg-sun-400 text-graphite-900",
        ok: "bg-sun-100 text-sun-700",
        bad: "bg-coral-100 text-coral-700",
        run: "bg-sky-100 text-sky-700",
        agent: "bg-graphite-900 text-sun-400"
      }.freeze

      def initialize(tone: :neutral, icon: nil, **html_options)
        @tone = tone
        @icon = icon
        @html_options = html_options
      end

      def call
        classes = class_names("inline-flex h-[26px] items-center gap-1.5 whitespace-nowrap rounded-full px-3 text-[12.5px] font-bold", TONES.fetch(@tone), @html_options[:class])
        tag.span safe_join([icon, content].compact), **@html_options.except(:class), class: classes
      end

      private

      def icon
        render Atoms::Icon::Component.new(name: @icon, class: "size-3.5") if @icon
      end
    end
  end
end
