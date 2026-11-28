module Atoms
  module StatusDot
    class Component < ViewComponent::Base
      TONES = {
        ok: "bg-lime",
        bad: "bg-danger-bright",
        none: "bg-forest-muted",
        run: "bg-info-soft",
        warn: "bg-warn"
      }.freeze

      def initialize(tone:, label:)
        @tone = tone
        @label = label
      end

      def call
        tag.span class: class_names("inline-block size-[7px] shrink-0 rounded-full", TONES.fetch(@tone)), role: "img", "aria-label": @label
      end
    end
  end
end
