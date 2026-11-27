module Atoms
  module Chip
    class Component < ViewComponent::Base
      TONES = {
        light: "bg-white text-ink",
        deep: "border-white/40 bg-brand-900 text-brand-200"
      }.freeze

      def initialize(tone: :light, upper: false, **html_options)
        @tone = tone
        @upper = upper
        @html_options = html_options
      end

      def call
        classes = class_names(
          "nb-chip",
          TONES.fetch(@tone),
          { "uppercase tracking-wider text-[11px]" => @upper },
          @html_options[:class]
        )
        tag.span content, **@html_options.except(:class), class: classes
      end
    end
  end
end
