module Molecules
  module Brand
    class Component < ViewComponent::Base
      TONES = {
        light: { text: "text-ink", dot: "bg-brand-500" },
        deep: { text: "text-white", dot: "bg-sun-400" },
        graphite: { text: "text-graphite-900", dot: "bg-sun-400" }
      }.freeze

      SIZES = {
        md: "text-[1.75rem]",
        lg: "text-[2.25rem]"
      }.freeze

      def initialize(tone: :light, size: :md)
        @tone = TONES.fetch(tone)
        @size = SIZES.fetch(size)
      end

      private
      attr_reader :tone, :size
    end
  end
end
