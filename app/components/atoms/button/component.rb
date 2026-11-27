module Atoms
  module Button
    class Component < ViewComponent::Base
      TONES = {
        light: "text-ink-soft hover:text-brand-600",
        deep: "text-brand-200 hover:text-white"
      }.freeze

      def initialize(href:, method: :post, tone: :light, **html_options)
        @href = href
        @method = method
        @tone = tone
        @html_options = html_options
      end

      def call
        helpers.button_to content, @href, method: @method, class: "text-sm font-bold cursor-pointer #{TONES.fetch(@tone)}", **@html_options
      end
    end
  end
end
