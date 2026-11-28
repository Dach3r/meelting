module Atoms
  module Link
    class Component < ViewComponent::Base
      TONES = {
        light: "font-semibold text-forest hover:text-ink",
        menu: "block rounded-xl px-4 py-2.5 font-semibold text-ink hover:bg-cream",
        underline: "font-semibold text-forest underline underline-offset-3 hover:text-ink"
      }.freeze

      def initialize(href:, tone: :light, **html_options)
        @href = href
        @tone = tone
        @html_options = html_options
      end

      def call
        link_to content, @href, class: class_names("text-sm", TONES.fetch(@tone), @html_options[:class]), **@html_options.except(:class)
      end
    end
  end
end
