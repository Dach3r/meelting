module Molecules
  module Notice
    class Component < ViewComponent::Base
      TONES = {
        alert: { classes: "border-coral-400/60 bg-coral-400/10 text-coral-100", role: "alert" },
        notice: { classes: "border-sky-400/60 bg-sky-400/10 text-sky-100", role: "status" }
      }.freeze

      def initialize(tone: :notice, **html_options)
        @tone = TONES.fetch(tone)
        @html_options = html_options
      end

      def call
        tag.div content, role: @tone[:role], class: class_names("mb-5 rounded-2xl border px-4 py-2.5 text-sm font-semibold", @tone[:classes], @html_options[:class]), **@html_options.except(:class)
      end
    end
  end
end
