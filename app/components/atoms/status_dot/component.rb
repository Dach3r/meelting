module Atoms
  module StatusDot
    class Component < ViewComponent::Base
      TONES = {
        ok: "bg-brand-500",
        bad: "bg-danger-500",
        none: "bg-graphite-300",
        run: "bg-sky-400",
        warn: "bg-amber-500"
      }.freeze

      def initialize(tone:, label:)
        @tone = tone
        @label = label
      end

      def call
        tag.span class: class_names("inline-block size-2 shrink-0 rounded-full", TONES.fetch(@tone)), role: "img", "aria-label": @label
      end
    end
  end
end
