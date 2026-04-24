import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
	try {
		const { email } = await request.json();

		if (!email || !email.includes('@')) {
			return json({ error: 'Valid email is required' }, { status: 400 });
		}

		// TODO: integrate with email service (Resend / Mailgun / SendGrid).
		// SECURITY: never log the raw email address — it is PII and our
		// application log retention window is not equal to our PII store's.
		// If a correlation signal is needed, log a SHA-256 prefix instead.
		if (import.meta.env.DEV) {
			console.log('[greeks-pdf] request received (dev only)');
		}

		// In production, you would:
		// 1. Add email to your mailing list (e.g., ConvertKit, Mailchimp)
		// 2. Send email with PDF attachment or download link
		// 3. Track conversion in analytics

		// Example with a hypothetical email service:
		// await emailService.send({
		//   to: email,
		//   subject: 'Your Free Options Greeks Guide',
		//   template: 'greeks-pdf',
		//   attachments: [{ filename: 'options-greeks-guide.pdf', path: '/pdfs/greeks.pdf' }]
		// });

		return json({
			success: true,
			message: 'PDF sent successfully'
		});
	} catch (error) {
		console.error('Greeks PDF request error:', error);
		return json({ error: 'Failed to process request. Please try again.' }, { status: 500 });
	}
};
