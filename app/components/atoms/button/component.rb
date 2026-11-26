module Atoms
  module Button
    class Component < ViewComponent::Base
      TONES = {
        light: "text-muted hover:text-ink",
        deep: "text-deep-muted hover:text-deep-ink"
      }.freeze

      def initialize(href:, method: :post, tone: :light, **html_options)
        @href = href
        @method = method
        @tone = tone
        @html_options = html_options
      end

      def call
        helpers.button_to content, @href, method: @method, class: "text-sm font-medium cursor-pointer #{TONES.fetch(@tone)}", **@html_options
      end
    end
  end
end
