module Atoms
  module Chip
    class Component < ViewComponent::Base
      TONES = {
        light: "border-line text-muted",
        deep: "border-deep-line text-deep-muted"
      }.freeze

      SHAPES = {
        square: "rounded-ui",
        pill: "rounded-full"
      }.freeze

      def initialize(tone: :light, shape: :square, upper: false, **html_options)
        @tone = tone
        @shape = shape
        @upper = upper
        @html_options = html_options
      end

      def call
        classes = class_names(
          "inline-flex items-center gap-1.5 border px-2.5 py-0.5 text-xs",
          TONES.fetch(@tone),
          SHAPES.fetch(@shape),
          { "uppercase tracking-wider font-semibold text-[11px]" => @upper },
          @html_options[:class]
        )
        tag.span content, **@html_options.except(:class), class: classes
      end
    end
  end
end
