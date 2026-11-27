module Molecules
  module Notice
    class Component < ViewComponent::Base
      TONES = {
        alert: { classes: "bg-coral-100 text-coral-700", role: "alert" },
        notice: { classes: "bg-sky-100 text-sky-700", role: "status" }
      }.freeze

      def initialize(tone: :notice, **html_options)
        @tone = TONES.fetch(tone)
        @html_options = html_options
      end

      def call
        tag.div content, role: @tone[:role], class: class_names("rounded-2xl px-4 py-2.5 text-sm font-semibold", @tone[:classes], @html_options[:class]), **@html_options.except(:class)
      end
    end
  end
end
