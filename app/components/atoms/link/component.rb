module Atoms
  module Link
    class Component < ViewComponent::Base
      TONES = {
        light: "text-muted hover:text-ink",
        deep: "text-deep-muted hover:text-deep-ink"
      }.freeze

      def initialize(href:, tone: :light, **html_options)
        @href = href
        @tone = tone
        @html_options = html_options
      end

      def call
        link_to content, @href, class: "text-sm font-medium #{TONES.fetch(@tone)}", **@html_options
      end
    end
  end
end
