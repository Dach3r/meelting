module Atoms
  module SubmitButton
    class Component < ViewComponent::Base
      def initialize(**html_options)
        @html_options = html_options
      end

      private
      attr_reader :html_options

      def classes
        class_names(
          "group flex h-14 w-full cursor-pointer items-center justify-between rounded-2xl bg-sun-400 p-2 text-base font-extrabold text-ink transition hover:brightness-105",
          html_options[:class]
        )
      end
    end
  end
end
