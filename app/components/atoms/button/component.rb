module Atoms
  module Button
    class Component < ViewComponent::Base
      TONES = {
        light: "font-semibold text-forest hover:text-ink",
        menu: "block w-full rounded-xl px-4 py-2.5 text-left font-semibold text-ink hover:bg-cream",
        pill: "inline-flex h-11 items-center rounded-full bg-forest px-5 text-[15px] font-bold text-lime hover:bg-forest-active"
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
