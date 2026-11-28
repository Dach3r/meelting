module Molecules
  module Brand
    class Component < ViewComponent::Base
      TONES = {
        light: { name: "text-white", brace: "text-lime", dot: "from-lime to-white", mark: "bg-lime" },
        dark: { name: "text-ink", brace: "text-forest", dot: "from-lime-deep to-ink", mark: "bg-lime-deep" }
      }.freeze

      def initialize(tone: :dark, compact: false)
        @tone = TONES.fetch(tone)
        @compact = compact
      end

      private
      attr_reader :tone, :compact

      def name_parts
        ApplicationHelper::APP_NAME.downcase.split("i", 2)
      end
    end
  end
end
