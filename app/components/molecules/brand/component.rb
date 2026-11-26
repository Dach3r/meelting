module Molecules
  module Brand
    class Component < ViewComponent::Base
      TONES = {
        light: { text: "text-ink", mark: "text-accent-bright" },
        deep: { text: "text-deep-ink", mark: "text-signal" }
      }.freeze

      def initialize(tone: :light)
        @tone = TONES.fetch(tone)
      end

      private
      attr_reader :tone
    end
  end
end
