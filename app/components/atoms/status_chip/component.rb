module Atoms
  module StatusChip
    class Component < ViewComponent::Base
      TONES = {
        neutral: "border-ink text-ink",
        ok: "border-lime bg-lime text-forest",
        bad: "border-danger text-danger",
        alert: "border-danger-soft bg-danger-soft text-danger",
        run: "border-transparent bg-cream text-ink"
      }.freeze

      def initialize(tone: :neutral, icon: nil, **html_options)
        @tone = tone
        @icon = icon
        @html_options = html_options
      end

      def call
        classes = class_names("inline-flex h-[30px] items-center gap-1.5 whitespace-nowrap rounded-full border-[1.5px] px-3 text-[13px] font-semibold", TONES.fetch(@tone), @html_options[:class])
        tag.span safe_join([icon, content].compact), **@html_options.except(:class), class: classes
      end

      private

      def icon
        render Atoms::Icon::Component.new(name: @icon, class: "size-3.5", "stroke-width": 2) if @icon
      end
    end
  end
end
