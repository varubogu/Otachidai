## Rental flow
bot-rental-request-start = Please describe your purpose for renting. You have 10 minutes.
bot-rental-purpose-label = Describe your purpose
bot-rental-answers-label = Answers to questions
bot-rental-answer-prefix = Answer
bot-rental-assigned = Room assigned! You now have access to { $channel }.
bot-rental-timeout = Your rental request has timed out. Please try again.
bot-rental-report = User { $user } joined a voice channel but did not submit a purpose within 10 minutes.
bot-rental-released = Room released. Thank you for using otachidai.
bot-rental-no-rooms = No rooms are currently available. Please try again later.
bot-rental-already-renting = You already have an active rental.
bot-rental-room-occupied = That room is currently occupied.

## Handoff
bot-handoff-prompt = The room host has left. Would you like to take over?
bot-handoff-accepted = { $user } is now the new room host.
bot-handoff-timeout = No one took over. The room has been released.
bot-handoff-take-over = Take Over

## Admin commands
admin-report-channel-registered = Report channel registered: { $channel }
admin-rental-button-registered = Rental button channel registered. Button posted in { $channel }.
admin-question-preset-saved = Question preset saved.
admin-question-preset-name-required = Please specify a preset name.
admin-question-preset-at-least-one = Specify at least one of question_1 through question_10.
admin-question-preset-not-found = The specified question preset was not found.
admin-room-registered = Room registered successfully.
admin-room-deleted = Room deleted successfully.
admin-room-not-found = No room found with those channel IDs.
admin-permission-denied = You must be a server administrator to use this command.
admin-room-at-least-one = At least one of text_channel_id or voice_channel_id is required.

## Help
help-title = otachidai Bot — Help
help-user = **User Commands**
    `/rent` — Start a rental request
    `/help` — Show this help
help-admin = **Admin Commands**
    `/register_report_channel` — Register the channel for timeout notifications
    `/register_rental_button_channel` — Register the channel where the rental button is posted
    `/register_question_preset` — Register a question preset for rooms
    `/register_room` — Register a room (text channel, voice channel, or both)
    `/delete_room` — Delete a registered room

## Rent button
rent-button-label = Request Room

## Errors
error-generic = An error occurred. Please try again.
error-db = A database error occurred. Please contact the bot operator.
