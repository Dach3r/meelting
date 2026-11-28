module Molecules
  module Notice
    class Component < ViewComponent::Base
      TONES = {
        alert: { classes: "bg-danger-soft text-danger", role: "alert" },
        notice: { classes: "bg-lime-soft text-forest", role: "status" }
      }.freeze

      def initialize(tone: :notice, **html_options)
        @tone = TONES.fetch(tone)
        @html_options = html_options
      end

      def call
        tag.div content, role: @tone[:role], class: class_names("rounded-xl px-4 py-3 text-[15px] font-semibold", @tone[:classes], @html_options[:class]), **@html_options.except(:class)
      end
    end
  end
end
