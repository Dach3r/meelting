module Atoms
  module LinkButton
    class Component < ViewComponent::Base
      BASE = "nb-btn".freeze

      VARIANTS = {
        primary: "rounded-full border-transparent bg-sun-400 font-semibold text-graphite-900 focus:outline-sun-400/40 active:outline-sun-400/40",
        secondary: "rounded-full border-transparent bg-graphite-900 font-semibold text-white",
        signal: "nb-btn-raised-light nb-btn-raised border-white bg-sun-400 text-ink focus:outline-sun-400/40 active:outline-sun-400/40",
        deep: "border-white/40 bg-transparent text-white hover:bg-white/10 focus:outline-white/30 active:outline-white/30",
        inverse: "nb-btn-raised bg-white text-brand-700 hover:bg-brand-50 focus:outline-white/30 active:outline-white/30",
        graphite: "rounded-full border-transparent bg-graphite-900 font-semibold text-white",
        graphite_outline: "rounded-full border-transparent bg-graphite-50 font-semibold text-graphite-900 hover:bg-graphite-100",
        graphite_soft: "rounded-full border-transparent bg-white font-semibold text-graphite-900 hover:bg-graphite-100",
        inverse_outline: "border-white bg-transparent text-white hover:bg-white/10 focus:outline-white/30 active:outline-white/30"
      }.freeze

      SIZES = {
        sm: "px-3.5 py-1.5 text-sm",
        pill: "px-5 py-[11px] text-sm leading-[normal]",
        cta: "px-[34px] py-[18px] text-[17px] leading-[normal]",
        block: "w-full px-6 py-4 text-base leading-[normal]",
        md: "px-4 py-2.5 text-sm",
        lg: "px-6 py-3.5 text-base"
      }.freeze

      def initialize(href:, variant: :primary, size: :md, **html_options)
        @href = href
        @variant = variant
        @size = size
        @extra_class = html_options.delete(:class)
        @html_options = html_options
      end

      def call
        link_to content, @href, class: class_names(BASE, VARIANTS.fetch(@variant), SIZES.fetch(@size), @extra_class), **@html_options
      end
    end
  end
end
