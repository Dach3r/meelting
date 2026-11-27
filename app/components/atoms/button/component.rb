module Atoms
  module Button
    class Component < ViewComponent::Base
      TONES = {
        light: "font-bold text-ink-soft hover:text-brand-600",
        deep: "font-bold text-brand-200 hover:text-white",
        pill: "rounded-full bg-graphite-900 px-5 py-[11px] font-semibold leading-[normal] text-white"
      }.freeze

      def initialize(href:, method: :post, tone: :light, **html_options)
        @href = href
        @method = method
        @tone = tone
        @html_options = html_options
      end

      def call
        helpers.button_to content, @href, method: @method, class: "text-sm cursor-pointer #{TONES.fetch(@tone)}", **@html_options
      end
    end
  end
end
