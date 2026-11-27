module Atoms
  module Link
    class Component < ViewComponent::Base
      TONES = {
        light: "text-ink-soft hover:text-brand-600",
        deep: "text-brand-200 hover:text-white",
        signal: "text-sun-400 underline hover:no-underline"
      }.freeze

      def initialize(href:, tone: :light, **html_options)
        @href = href
        @tone = tone
        @html_options = html_options
      end

      def call
        link_to content, @href, class: class_names("text-sm font-bold", TONES.fetch(@tone), @html_options[:class]), **@html_options.except(:class)
      end
    end
  end
end
